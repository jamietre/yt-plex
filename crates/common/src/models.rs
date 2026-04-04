use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Downloading,
    Copying,
    Done,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Downloading => "downloading",
            Self::Copying => "copying",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for JobStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "downloading" => Ok(Self::Downloading),
            "copying" => Ok(Self::Copying),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            other => Err(anyhow::anyhow!("unknown job status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub url: String,
    pub status: JobStatus,
    pub channel_name: Option<String>,
    pub title: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sent over WebSocket to all connected clients on job status change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub job_id: String,
    pub status: JobStatus,
    pub channel_name: Option<String>,
    pub title: Option<String>,
    pub error: Option<String>,
}

impl WsMessage {
    pub fn from_job(job: &Job) -> Self {
        Self {
            job_id: job.id.clone(),
            status: job.status.clone(),
            channel_name: job.channel_name.clone(),
            title: job.title.clone(),
            error: job.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_roundtrips_as_str() {
        assert_eq!(JobStatus::Queued.as_str(), "queued");
        assert_eq!(
            "downloading".parse::<JobStatus>().unwrap(),
            JobStatus::Downloading
        );
        assert!("bogus".parse::<JobStatus>().is_err());
    }

    #[test]
    fn ws_message_serialises() {
        let msg = WsMessage {
            job_id: "abc".into(),
            status: JobStatus::Done,
            channel_name: Some("Chan".into()),
            title: Some("Vid".into()),
            error: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"status\":\"done\""));
    }
}
