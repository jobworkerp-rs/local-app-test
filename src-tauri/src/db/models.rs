use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub id: i64,
    pub worktree_base_path: String,
    pub default_base_branch: String,
    pub agent_timeout_minutes: i32,
    pub sync_interval_minutes: i32,
    pub grpc_server_url: String,
    pub locale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Platform {
    GitHub,
    Gitea,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::GitHub => write!(f, "GitHub"),
            Platform::Gitea => write!(f, "Gitea"),
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GitHub" => Ok(Platform::GitHub),
            "Gitea" => Ok(Platform::Gitea),
            _ => Err(format!("Unknown platform: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub mcp_server_name: String,
    pub platform: Platform,
    pub base_url: String,
    pub name: String,
    pub url: String,
    pub owner: String,
    pub repo_name: String,
    pub local_path: Option<String>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepository {
    pub mcp_server_name: String,
    pub platform: Platform,
    pub base_url: String,
    pub name: String,
    pub url: String,
    pub owner: String,
    pub repo_name: String,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentJobStatus {
    Pending,
    PreparingWorkspace,
    FetchingIssue,
    RunningAgent,
    CreatingPR,
    PrCreated,
    Merged,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for AgentJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentJobStatus::Pending => write!(f, "Pending"),
            AgentJobStatus::PreparingWorkspace => write!(f, "PreparingWorkspace"),
            AgentJobStatus::FetchingIssue => write!(f, "FetchingIssue"),
            AgentJobStatus::RunningAgent => write!(f, "RunningAgent"),
            AgentJobStatus::CreatingPR => write!(f, "CreatingPR"),
            AgentJobStatus::PrCreated => write!(f, "PrCreated"),
            AgentJobStatus::Merged => write!(f, "Merged"),
            AgentJobStatus::Completed => write!(f, "Completed"),
            AgentJobStatus::Failed => write!(f, "Failed"),
            AgentJobStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::str::FromStr for AgentJobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(AgentJobStatus::Pending),
            "PreparingWorkspace" => Ok(AgentJobStatus::PreparingWorkspace),
            "FetchingIssue" => Ok(AgentJobStatus::FetchingIssue),
            "RunningAgent" => Ok(AgentJobStatus::RunningAgent),
            "CreatingPR" => Ok(AgentJobStatus::CreatingPR),
            "PrCreated" => Ok(AgentJobStatus::PrCreated),
            "Merged" => Ok(AgentJobStatus::Merged),
            "Completed" => Ok(AgentJobStatus::Completed),
            "Failed" => Ok(AgentJobStatus::Failed),
            "Cancelled" => Ok(AgentJobStatus::Cancelled),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJob {
    pub id: i64,
    pub repository_id: i64,
    pub issue_number: i32,
    pub jobworkerp_job_id: String,
    pub status: AgentJobStatus,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub pr_number: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentJob {
    pub repository_id: i64,
    pub issue_number: i32,
    pub jobworkerp_job_id: String,
}
