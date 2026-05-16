//! Agent-side image transfer state: receives chunk frames, verifies each
//! chunk's SHA-256, reassembles them into a temp file, and loads the result
//! into Docker on commit.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

use tokio::sync::Mutex;

use icefall_common::transfer::{hex_encode, sha256_hex, sha256_of, ChunkFrame};

/// One in-progress image transfer.
struct Transfer {
    file: std::fs::File,
    path: std::path::PathBuf,
    total_chunks: u32,
    /// Which chunk indices have been received and verified.
    received: Vec<bool>,
    chunk_size: usize,
    /// Expected SHA-256 (hex) of the fully reassembled file.
    total_sha256: String,
}

#[derive(Clone, Default)]
pub struct TransferManager {
    transfers: Arc<Mutex<HashMap<[u8; 16], Transfer>>>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new transfer: allocate a temp file sized for `total_chunks`.
    pub async fn begin(
        &self,
        transfer_id: [u8; 16],
        total_chunks: u32,
        total_sha256: String,
        chunk_size: usize,
    ) -> Result<(), String> {
        let path =
            std::env::temp_dir().join(format!("icefall-img-{}.tar.gz", hex_encode(&transfer_id)));
        let file = std::fs::File::create(&path).map_err(|e| format!("temp file: {e}"))?;

        let mut transfers = self.transfers.lock().await;
        transfers.insert(
            transfer_id,
            Transfer {
                file,
                path,
                total_chunks,
                received: vec![false; total_chunks as usize],
                chunk_size,
                total_sha256,
            },
        );
        Ok(())
    }

    /// Write a verified chunk at its indexed offset. Returns `Ok(true)` if the
    /// chunk was accepted, `Ok(false)` if it failed verification (caller should
    /// nack so the sender retries), `Err` for protocol/IO errors.
    pub async fn accept_chunk(&self, frame: &ChunkFrame) -> Result<bool, String> {
        if !frame.verify() {
            return Ok(false);
        }
        let mut transfers = self.transfers.lock().await;
        let transfer = transfers
            .get_mut(&frame.transfer_id)
            .ok_or_else(|| "unknown transfer_id".to_string())?;

        if frame.chunk_index >= transfer.total_chunks {
            return Err("chunk_index out of range".to_string());
        }

        let offset = frame.chunk_index as u64 * transfer.chunk_size as u64;
        transfer
            .file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| format!("seek: {e}"))?;
        transfer
            .file
            .write_all(&frame.payload)
            .map_err(|e| format!("write: {e}"))?;
        transfer.received[frame.chunk_index as usize] = true;
        Ok(true)
    }

    /// Finalize a transfer: verify all chunks arrived and the whole-file digest
    /// matches. Returns the temp file path on success. The caller is
    /// responsible for `docker load` and cleanup via [`Self::cleanup`].
    pub async fn commit(&self, transfer_id: [u8; 16]) -> Result<std::path::PathBuf, String> {
        let mut transfers = self.transfers.lock().await;
        let transfer = transfers
            .get_mut(&transfer_id)
            .ok_or_else(|| "unknown transfer_id".to_string())?;

        if let Some(missing) = transfer.received.iter().position(|r| !r) {
            return Err(format!("missing chunk {missing}"));
        }
        transfer.file.flush().map_err(|e| format!("flush: {e}"))?;

        let bytes = std::fs::read(&transfer.path).map_err(|e| format!("read for verify: {e}"))?;
        let digest = sha256_hex(&sha256_of(&bytes));
        if digest != transfer.total_sha256 {
            return Err(format!(
                "whole-file checksum mismatch: expected {}, got {digest}",
                transfer.total_sha256
            ));
        }
        Ok(transfer.path.clone())
    }

    /// Remove a transfer's temp file and drop its state.
    pub async fn cleanup(&self, transfer_id: [u8; 16]) {
        let mut transfers = self.transfers.lock().await;
        if let Some(transfer) = transfers.remove(&transfer_id) {
            let _ = std::fs::remove_file(&transfer.path);
        }
    }
}
