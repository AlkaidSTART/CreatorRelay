use chrono::{DateTime, Datelike, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

use crate::{
    content::{self, ContentAsset, ContentKind},
    ecosystem, platforms,
    plugins::{
        self, CapabilityLevel, PluginRuntime, PublishInput, PublishMediaRef,
        PublishValidationResult, validate_publish_payload,
    },
};


mod analytics;
mod dashboard;
mod external_works;
mod manual_package;
mod publish;
mod types;

pub use analytics::{AnalyticsGranularity, AnalyticsPoint, AnalyticsQuery, AnalyticsResult, AnalyticsSeries, analytics_trend};
pub use dashboard::{DashboardFilter, DashboardFilterApplied, OperationsDashboard, operations_dashboard};
pub use external_works::{
    ExternalWork, ExternalWorkFilter, MetricSnapshot, SyncPlatformWorksInput, SyncRunResult,
    external_work, list_external_works, list_sync_runs, sync_platform_works,
};
pub use manual_package::export_manual_publish_package;
pub use publish::{
    CreatePublishTasksInput, CreatePublishTasksResult, ManualCompletionInput, ManualPackageAsset,
    ManualPublishPackage, PreviewAction, PublishExecutionMode, PublishPreview, PublishSourceType,
    PublishTargetInput, PublishTask, PublishTaskEvent, PublishTaskStatus, ResolveUncertainInput,
    UncertainOutcome, cancel_publish_task, complete_manual_publish_task, confirm_publish_task,
    create_publish_tasks, list_publish_task_events, list_publish_tasks, prepare_publish_task,
    publish_task, resolve_uncertain_publish_task, resume_account_tasks, retry_publish_task,
    run_due_publish_tasks, update_publish_task_schedule,
};
