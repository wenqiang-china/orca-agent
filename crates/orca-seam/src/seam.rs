use crate::anchor::{AnchorKeeper, AnchorType};
use chrono::{Duration, Utc};
use orca_utils::message::{Message, Role};
use serde::{Deserialize, Serialize};

/// Result of a compression operation
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Number of messages compressed
    pub messages_compressed: usize,
    /// Number of summary messages created
    pub summaries_created: usize,
    /// Estimated tokens saved
    pub tokens_saved: usize,
}

/// An archive layer defines compression parameters for messages of a certain age
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveLayer {
    /// Name for this layer
    pub name: String,
    /// Messages older than this get compressed
    pub age_threshold: Duration,
    /// Target compression ratio (0.0 - 1.0, lower = more compressed)
    pub compression_ratio: f32,
    /// Maximum content length for messages in this layer
    pub max_content_len: usize,
}

/// Seam Manager handles layered context archiving
pub struct SeamManager {
    layers: Vec<ArchiveLayer>,
    anchor_keeper: AnchorKeeper,
    /// Total characters saved by compression
    total_chars_saved: usize,
}

impl SeamManager {
    pub fn new(max_anchors: usize) -> Self {
        let layers = vec![
            ArchiveLayer {
                name: "recent".to_string(),
                age_threshold: Duration::minutes(10),
                compression_ratio: 1.0, // No compression
                max_content_len: usize::MAX,
            },
            ArchiveLayer {
                name: "warm".to_string(),
                age_threshold: Duration::minutes(30),
                compression_ratio: 0.5,
                max_content_len: 2000,
            },
            ArchiveLayer {
                name: "cool".to_string(),
                age_threshold: Duration::hours(2),
                compression_ratio: 0.3,
                max_content_len: 500,
            },
            ArchiveLayer {
                name: "cold".to_string(),
                age_threshold: Duration::hours(24),
                compression_ratio: 0.1,
                max_content_len: 200,
            },
        ];

        Self {
            layers,
            anchor_keeper: AnchorKeeper::new(max_anchors),
            total_chars_saved: 0,
        }
    }

    /// Compress messages that are older than their layer's threshold
    pub fn compress(&mut self, messages: &mut Vec<Message>) -> CompressionResult {
        let now = Utc::now();
        let mut compressed_count = 0;
        let mut summaries_created = 0;
        let mut chars_saved = 0;

        // Don't compress the last 6 messages (keep recent context intact)
        let protect_count = 6.min(messages.len());
        let compressible_end = messages.len().saturating_sub(protect_count);

        // Auto-extract anchors before compression (clone data to avoid borrow conflicts)
        let compressible_messages: Vec<Message> = messages[..compressible_end].to_vec();
        self.extract_anchors(&compressible_messages);

        for msg in messages.iter_mut().take(compressible_end) {
            let age = now.signed_duration_since(msg.timestamp);

            // Find the appropriate layer
            if let Some(layer) = self.find_layer(age)
                && layer.compression_ratio < 1.0 && msg.content.len() > layer.max_content_len
            {
                let original_len = msg.content.len();
                let compressed = self.compress_message_content(&msg.content, layer);
                chars_saved += original_len - compressed.len();
                msg.content = compressed;
                compressed_count += 1;
            }
        }

        // Create summary message if significant compression happened
        if chars_saved > 1000 {
            let summary = self.create_summary_message(messages, chars_saved);
            // Insert summary at position 0 (beginning)
            messages.insert(0, summary);
            summaries_created += 1;
        }

        self.total_chars_saved += chars_saved;

        CompressionResult {
            messages_compressed: compressed_count,
            summaries_created,
            tokens_saved: chars_saved / 4, // rough estimate: 4 chars per token
        }
    }

    /// Find the archive layer for a given age
    fn find_layer(&self, age: Duration) -> Option<&ArchiveLayer> {
        // Find the tightest layer that still applies
        self.layers
            .iter()
            .rev()
            .find(|layer| age >= layer.age_threshold)
    }

    /// Compress a single message's content
    fn compress_message_content(&self, content: &str, layer: &ArchiveLayer) -> String {
        if content.len() <= layer.max_content_len {
            return content.to_string();
        }

        // Truncate with ellipsis
        let target_len = (content.len() as f32 * layer.compression_ratio) as usize;
        let target_len = target_len.min(layer.max_content_len);

        if target_len < 50 {
            // Too compressed, just keep a note
            return format!("[Compressed message, originally {} chars]", content.len());
        }

        // Try to cut at a sentence boundary
        let cut_point = content[..target_len]
            .rfind(". ")
            .or_else(|| content[..target_len].rfind('\n'))
            .unwrap_or(target_len);

        format!("{}... [{} chars total]", &content[..cut_point], content.len())
    }

    /// Extract anchors from messages that are about to be compressed
    fn extract_anchors(&mut self, messages: &[Message]) {
        for msg in messages {
            // Extract user goals from user messages
            if msg.role == Role::User {
                let content = msg.content.trim();
                if !content.is_empty() && content.len() < 500 {
                    // Heuristic: short user messages are likely goals/instructions
                    self.anchor_keeper.add(AnchorType::UserGoal, content);
                }
            }
        }
    }

    /// Create a summary message from compression context
    fn create_summary_message(&self, messages: &[Message], chars_saved: usize) -> Message {
        let total = messages.len();
        let tool_calls = messages.iter().filter(|m| !m.tool_calls.is_empty()).count();
        let summary = format!(
            "[Context Archive Summary: {} messages, {} tool calls, ~{} chars compressed]\n\n{}",
            total,
            tool_calls,
            chars_saved,
            self.anchor_keeper.format_as_context()
        );
        Message::system(summary)
    }

    /// Get the anchor keeper
    pub fn anchors(&self) -> &AnchorKeeper {
        &self.anchor_keeper
    }

    /// Get a mutable reference to the anchor keeper
    pub fn anchors_mut(&mut self) -> &mut AnchorKeeper {
        &mut self.anchor_keeper
    }

    /// Total characters saved across all compressions
    pub fn total_chars_saved(&self) -> usize {
        self.total_chars_saved
    }

    /// Configure custom layers
    pub fn set_layers(&mut self, layers: Vec<ArchiveLayer>) {
        self.layers = layers;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_messages(count: usize) -> Vec<Message> {
        let now = Utc::now();
        (0..count)
            .map(|i| {
                let mut msg = Message::user(format!("Message {} with some content to compress. This is a longer message that has enough text to be worth compressing down in the cooler layers of the archive. The content goes on and on to fill space.", i));
                // Make older messages older
                msg.timestamp = now - Duration::minutes((count - i) as i64 * 15);
                msg
            })
            .collect()
    }

    #[test]
    fn test_compress_preserves_recent() {
        let mut seam = SeamManager::new(100);
        let mut msgs = make_messages(10);
        let result = seam.compress(&mut msgs);
        // Recent messages (last 6) should not be compressed
        // Older messages may be compressed
        assert!(result.messages_compressed <= 4);
    }

    #[test]
    fn test_anchor_extraction() {
        let mut seam = SeamManager::new(100);
        let msgs = make_messages(3);
        seam.extract_anchors(&msgs);
        // Should extract user goals from user messages
        assert!(seam.anchor_keeper.count() > 0);
    }

    #[test]
    fn test_custom_layers() {
        let mut seam = SeamManager::new(100);
        seam.set_layers(vec![
            ArchiveLayer {
                name: "fast".to_string(),
                age_threshold: Duration::seconds(0),
                compression_ratio: 0.5,
                max_content_len: 100,
            },
        ]);
        let mut msgs = make_messages(10);
        let result = seam.compress(&mut msgs);
        // With aggressive settings, should compress more
        assert!(result.messages_compressed > 0);
    }
}