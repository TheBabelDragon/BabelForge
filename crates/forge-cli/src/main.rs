//! BabelForge CLI — `forge`
//!
//! Commands: version, capabilities, and the Phase 7 experiment.

use clap::{Parser, Subcommand};
use forge_contracts::{OperationRequest, OperationStatus};
use forge_executor::{counter_capability, sensor_capability};
use forge_node::{build_experiment_nodes, Node};
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "forge", version, about = "BabelForge — Universal Execution Runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Version,
    Experiment {
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Capabilities,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("forge {}", forge_core::VERSION);
            println!("contracts {}", forge_contracts::CONTRACT_VERSION);
            Ok(())
        }
        Commands::Capabilities => {
            let c = counter_capability();
            let s = sensor_capability();
            println!("{}", serde_json::to_string_pretty(&vec![c, s])?);
            Ok(())
        }
        Commands::Experiment { data_dir } => run_experiment(data_dir),
    }
}

fn run_experiment(data_dir: Option<PathBuf>) -> anyhow::Result<()> {
    println!("=== BabelForge v0.1 Phase 7 Experiment ===\n");

    let tmp = tempfile::tempdir()?;
    let base = data_dir.unwrap_or_else(|| tmp.path().to_path_buf());
    std::fs::create_dir_all(&base)?;

    let counter_db = base.join("counter.db");
    let sensor_db = base.join("sensor.db");

    let (mut node_a, mut node_b) =
        build_experiment_nodes(Some(&counter_db), Some(&sensor_db))?;

    println!("1. Nodes registered:");
    println!("   A: {} ({})", node_a.id.name, node_a.id.id);
    println!("   B: {} ({})", node_b.id.name, node_b.id.id);

    println!("\n2. Capabilities discoverable:");
    for cap in node_a.executor.list_capabilities() {
        println!("   A offers {}", cap.id);
    }
    for cap in node_b.executor.list_capabilities() {
        println!("   B offers {}", cap.id);
    }

    let snap_a = node_a.inspect_state(Some("counter.v1"))?;
    println!("\n3. Initial state A (counter): {}", snap_a.state);

    let req_inc = OperationRequest::new(
        node_a.id.id,
        "counter.v1",
        json!({ "op": "increment", "delta": 3 }),
        node_a.id.id,
    );
    let res_inc = node_a.submit(req_inc)?;
    println!("\n4. Authorized increment → {:?}", res_inc.status);
    println!("   output: {:?}", res_inc.output);

    let stranger = Uuid::new_v4();
    let req_bad = OperationRequest::new(
        node_a.id.id,
        "counter.v1",
        json!({ "op": "increment", "delta": 1 }),
        stranger,
    );
    let res_bad = node_a.submit(req_bad)?;
    println!("\n5. Unauthorized → {:?}", res_bad.status);
    assert_eq!(res_bad.status, OperationStatus::Rejected);

    let mut req_budget = OperationRequest::new(
        node_a.id.id,
        "counter.v1",
        json!({ "op": "increment", "delta": 1 }),
        node_a.id.id,
    );
    req_budget.budget = Some(0);
    let res_limit = node_a.submit(req_budget);
    println!("\n6. Resource limit (budget 0): {:?}", res_limit.map(|r| r.status));

    let events = node_a.events(Some(100))?;
    println!("\n7. Events recorded on A: {}", events.len());

    let id_before = node_a.id.id;
    drop(node_a);
    let mut node_a2 = Node::create("counter-node", Some(&counter_db))?;
    forge_executor::register_counter(&mut node_a2.executor);
    node_a2.publish();
    node_a2.grant(node_a2.id.id, "counter.v1");

    println!("\n8. After restart:");
    println!("   identity preserved: {} (was {})", node_a2.id.id, id_before);
    assert_eq!(node_a2.id.id, id_before, "node UUID must survive reopen");
    let val = node_a2
        .store
        .get_state("counter.v1")?
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    println!("   counter value from DB: {}", val);
    assert_eq!(val, 3, "state must survive process restart");

    let integrity = node_a2.verify_integrity()?;
    println!("\n9. Event chain integrity: {}", integrity);
    assert!(integrity, "event chain must verify");

    let notes = node_a2.replay_summary()?;
    println!("\n10. Replay notes ({}):", notes.len());
    for n in notes.iter().take(5) {
        println!("    {}", n);
    }

    let mut req1 = OperationRequest::new(
        node_a2.id.id,
        "counter.v1",
        json!({ "op": "increment", "delta": 2 }),
        node_a2.id.id,
    );
    req1.idempotency_key = Some("exp-idem-1".into());
    let r1 = node_a2.submit(req1)?;
    let mut req2 = OperationRequest::new(
        node_a2.id.id,
        "counter.v1",
        json!({ "op": "increment", "delta": 2 }),
        node_a2.id.id,
    );
    req2.idempotency_key = Some("exp-idem-1".into());
    let r2 = node_a2.submit(req2)?;
    println!("\n11. Idempotency:");
    println!("    first:  {:?}", r1.status);
    println!("    second: {:?} (from_replay={})", r2.status, r2.from_replay);
    assert_eq!(r2.status, OperationStatus::IdempotentReplay);

    // Conflict: same key, different input
    let mut req3 = OperationRequest::new(
        node_a2.id.id,
        "counter.v1",
        json!({ "op": "increment", "delta": 5 }),
        node_a2.id.id,
    );
    req3.idempotency_key = Some("exp-idem-1".into());
    let conflict = node_a2.submit(req3);
    println!("    conflict (different input, same key): {:?}", conflict.as_ref().err().map(|e| e.code.as_str()));
    assert!(conflict.is_err(), "different request must conflict on same idempotency key");

    let req_s = OperationRequest::new(
        node_b.id.id,
        "sensor.threshold.v1",
        json!({ "op": "threshold", "threshold": 40.0 }),
        node_b.id.id,
    );
    let rs = node_b.submit(req_s)?;
    println!("\nSensor threshold check: {:?}", rs.output);

    println!("\n=== Experiment complete — acceptance criteria exercised ===");
    Ok(())
}
