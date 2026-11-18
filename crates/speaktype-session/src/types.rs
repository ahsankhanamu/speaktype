use serde::{Deserialize, Serialize};

/// A timeline item inside a session.
///
/// Serde names are the IPC vocabulary ("speech" | "mark" | …); the DB column
/// `event_type` keeps the legacy values (transcript | mark | screenshot |
/// clipboard | note) per spec §12.
#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Speech,
    Mark,
    Screenshot,
    Clip,
    Note,
}

/// Where a session's rendered prompt is headed.
#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    CodingAgent,
    Chat,
    Markdown,
}

#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: i64,
    pub name: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub template: String,
    pub target: Target,
    pub preface: Option<String>,
    pub bundle_path: Option<String>,
}

#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: i64,
    pub name: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub mark_count: u32,
    pub clip_count: u32,
    pub snippet: Option<String>,
}

#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: i64,
    pub session_id: i64,
    pub kind: ItemKind,
    pub ord: f64,
    pub t_start_ms: i64,
    pub t_end_ms: Option<i64>,
    pub text: Option<String>,
    pub caption: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewItem {
    pub kind: ItemKind,
    pub t_start_ms: i64,
    pub t_end_ms: Option<i64>,
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemPatch {
    pub text: Option<String>,
    pub caption: Option<String>,
}

#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session: Session,
    pub items: Vec<Item>,
}

/// A rectangle in display points (mark bounds, from the overlay window).
#[derive(Serialize, Deserialize, specta::Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
