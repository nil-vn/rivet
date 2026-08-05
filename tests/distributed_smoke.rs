#![doc = "Ballista scheduler/executor loopback smoke tests."]
#![cfg(feature = "distributed")]

use std::time::Duration;

use ballista::prelude::{SessionConfigExt, SessionContextExt};
use datafusion::arrow::array::Int64Array;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::context::SessionConfig;
use datafusion::execution::session_state::SessionStateBuilder;
use datafusion::physical_plan::collect;
use datafusion::prelude::SessionContext;
use tokio::time::timeout;

const SMOKE_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standalone_cluster_executes_an_aggregate_query() -> Result<()> {
    let batches = timeout(SMOKE_TIMEOUT, async {
        let config = SessionConfig::new_with_ballista().with_ballista_standalone_parallelism(2);
        let state = SessionStateBuilder::new()
            .with_config(config)
            .with_default_features()
            .build();
        let context = SessionContext::standalone_with_state(state).await?;

        let dataframe = context
            .sql(
                "SELECT SUM(value) AS total \
                 FROM (VALUES (1), (2), (3), (4)) AS source(value)",
            )
            .await?;
        let plan = dataframe.create_physical_plan().await?;

        if plan.name() != "DistributedQueryExec" {
            return Err(DataFusionError::Plan(format!(
                "expected DistributedQueryExec, received {}",
                plan.name()
            )));
        }

        collect(plan, context.task_ctx()).await
    })
    .await
    .map_err(|error| {
        DataFusionError::Execution(format!(
            "Ballista standalone smoke test exceeded {SMOKE_TIMEOUT:?}: {error}"
        ))
    })??;

    let batch = batches.first().ok_or_else(|| {
        DataFusionError::Execution("Ballista returned no record batches".to_owned())
    })?;
    let totals = batch
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .ok_or_else(|| DataFusionError::Execution("aggregate result was not Int64".to_owned()))?;

    assert_eq!(totals.len(), 1);
    assert_eq!(totals.value(0), 10);

    Ok(())
}
