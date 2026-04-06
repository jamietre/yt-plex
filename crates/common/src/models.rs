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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube_id: Option<String>,
}

impl WsMessage {
    pub fn from_job(job: &Job) -> Self {
        Self {
            job_id: job.id.clone(),
            status: job.status.clone(),
            channel_name: job.channel_name.clone(),
            title: job.title.clone(),
            error: job.error.clone(),
            progress: None,
            youtube_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub youtube_channel_url: String,
    pub name: String,
    pub last_synced_at: Option<String>,
    /// YouTube channel ID in UCxxxxxxxxxxxxxxxx format.
    /// Populated on first sync; used in the {channel_id} path template variable.
    pub youtube_channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoStatus {
    New,
    InProgress,
    Downloaded,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub youtube_id: String,
    pub channel_id: String,
    pub title: String,
    pub published_at: Option<String>,
    pub downloaded_at: Option<String>,
    pub last_seen_at: String,
    pub ignored_at: Option<String>,
    pub status: VideoStatus,
    pub description: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub linked_email: Option<String>,
    pub is_admin_profile: bool,
    pub created_at: String,
}

/// Paginated response for list_videos_for_channel.
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoPage {
    pub videos: Vec<Video>,
    pub has_more: bool,
}

/// Filter parameter for list_videos_for_channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoFilter {
    New,
    Downloaded,
    All,
}

impl VideoFilter {
    pub fn parse(s: &str) -> Self {
        match s {
            "downloaded" => Self::Downloaded,
            "all" => Self::All,
            _ => Self::New,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Downloaded => "downloaded",
            Self::All => "all",
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
            progress: None,
            youtube_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"status\":\"done\""));
        assert!(!json.contains("progress"));
        assert!(!json.contains("youtube_id"));

        let msg_with_progress = WsMessage {
            job_id: "abc".into(),
            status: JobStatus::Downloading,
            channel_name: None,
            title: None,
            error: None,
            progress: Some(42.5),
            youtube_id: None,
        };
        let json2 = serde_json::to_string(&msg_with_progress).unwrap();
        assert!(json2.contains("\"progress\":42.5"));
    }

    #[test]
    fn video_status_serialises() {
        let s = serde_json::to_string(&VideoStatus::New).unwrap();
        assert_eq!(s, "\"new\"");
        let s2 = serde_json::to_string(&VideoStatus::InProgress).unwrap();
        assert_eq!(s2, "\"in_progress\"");
    }

    #[test]
    fn ws_message_includes_youtube_id_when_set() {
        let msg = WsMessage {
            job_id: "j1".into(),
            status: JobStatus::Done,
            channel_name: None,
            title: None,
            error: None,
            progress: None,
            youtube_id: Some("abc123".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"youtube_id\":\"abc123\""));
    }

    #[test]
    fn ws_message_omits_youtube_id_when_none() {
        let msg = WsMessage {
            job_id: "j1".into(),
            status: JobStatus::Done,
            channel_name: None,
            title: None,
            error: None,
            progress: None,
            youtube_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("youtube_id"));
    }

    #[test]
    fn video_has_description_field() {
        let v = Video {
            youtube_id: "abc".into(),
            channel_id: "ch1".into(),
            title: "Title".into(),
            published_at: None,
            downloaded_at: None,
            last_seen_at: "2026-04-05T00:00:00Z".into(),
            ignored_at: None,
            status: VideoStatus::New,
            description: Some("A description".into()),
            file_path: None,
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("\"description\":\"A description\""));
    }

    #[test]
    fn video_page_serialises() {
        let page = VideoPage {
            videos: vec![],
            has_more: false,
        };
        let json = serde_json::to_string(&page).unwrap();
        assert!(json.contains("\"has_more\":false"));
    }
}
