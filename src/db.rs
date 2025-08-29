use rusqlite::{params, Connection, OptionalExtension, NO_PARAMS};
use std::path::Path;
use crate::crypto::{derive_key, derive_key_with_config, encrypt, decrypt, Argon2Config};

/// SecureDb wraps an SQLite database. If the SQLite build is linked with SQLCipher,
/// we set the DB key so the file is encrypted at rest. If SQLCipher isn't available,
/// SecureDb stores application-encrypted blobs in a simple table.
pub struct SecureDb {
	conn: Connection,
	sqlcipher: bool,
}

impl SecureDb {
	/// Open a database path and attempt to enable SQLCipher if present.
	pub fn open(path: &Path, master_password: &str) -> rusqlite::Result<Self> {
		let conn = Connection::open(path)?;

		// Detect SQLCipher: PRAGMA cipher_version returns a string when present.
		let cipher_version: Option<String> = conn
			.query_row("PRAGMA cipher_version;", NO_PARAMS, |r| r.get(0))
			.optional()?;

		if let Some(_) = cipher_version {
			// SQLCipher present: derive a key and set PRAGMA key
			let salt = b"passman-sqlcipher-salt-2025";
			let key = derive_key_with_config(master_password, salt, &Argon2Config::default());
			let hex_key = hex::encode(key);
			// Set the key pragma to configure SQLCipher for the connection
			conn.pragma_update(None, "key", &hex_key)?;
			// Create a simple table for metadata
			conn.execute_batch(
				"CREATE TABLE IF NOT EXISTS metadata (k TEXT PRIMARY KEY, v BLOB);",
			)?;
			Ok(Self { conn, sqlcipher: true })
		} else {
			// No SQLCipher; create a table for encrypted blobs
			conn.execute_batch(
				"CREATE TABLE IF NOT EXISTS encrypted_blob (k TEXT PRIMARY KEY, v BLOB);",
			)?;
			Ok(Self { conn, sqlcipher: false })
		}
	}

	/// Put value into DB. If SQLCipher is enabled the DB file is encrypted; otherwise
	/// encrypt the value at application layer and store a salt + ciphertext blob.
	pub fn put(&self, key: &str, plaintext: &[u8], master_password: &str) -> rusqlite::Result<()> {
		if self.sqlcipher {
			self.conn.execute(
				"REPLACE INTO metadata (k, v) VALUES (?1, ?2)",
				params![key, plaintext],
			)?;
		} else {
			// Use random salt per entry
			let mut salt = vec![0u8; 32];
			getrandom::getrandom(&mut salt).expect("OS RNG failed");
			let derived = derive_key(master_password, &salt);
			let ct = encrypt(&derived, plaintext);
			let mut blob = salt;
			blob.extend_from_slice(&ct);
			self.conn.execute(
				"REPLACE INTO encrypted_blob (k, v) VALUES (?1, ?2)",
				params![key, blob],
			)?;
		}
		Ok(())
	}

	/// Get value for key. Returns plaintext when decryption succeeds.
	pub fn get(&self, key: &str, master_password: &str) -> rusqlite::Result<Option<Vec<u8>>> {
		if self.sqlcipher {
			let row: Option<Vec<u8>> = self.conn.query_row(
				"SELECT v FROM metadata WHERE k = ?1",
				params![key],
				|r| r.get(0),
			).optional()?;
			Ok(row)
		} else {
			let row: Option<Vec<u8>> = self.conn.query_row(
				"SELECT v FROM encrypted_blob WHERE k = ?1",
				params![key],
				|r| r.get(0),
			).optional()?;

			if let Some(blob) = row {
				if blob.len() <= 32 {
					return Ok(None);
				}
				let salt = &blob[0..32];
				let ct = &blob[32..];
				let derived = derive_key(master_password, salt);
				match decrypt(&derived, ct) {
					Ok(pt) => Ok(Some(pt)),
					Err(_) => Ok(None),
				}
			} else {
				Ok(None)
			}
		}
	}
}

fn _generate_entry_salt(key: &str) -> Vec<u8> {
	// Deterministic salt per key (only used if needed). Prefer random salt stored with blob.
	let mut s = vec![0u8; 32];
	let h = blake3::hash(key.as_bytes());
	let bytes = h.as_bytes();
	s.copy_from_slice(&bytes[0..32]);
	s
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::env;
	use std::fs;
	use uuid::Uuid;

	#[test]
	fn test_securedb_put_get_roundtrip() {
		let tmp = env::temp_dir();
		let fname = tmp.join(format!("passman_test_{}.db", Uuid::new_v4()));
		let master = "test_master_password";

		// Ensure file doesn't exist
		let _ = fs::remove_file(&fname);

		let db = SecureDb::open(&fname, master).expect("open db");

		let key = "service1";
		let data = b"super secret data";

		db.put(key, data, master).expect("put");

		let got = db.get(key, master).expect("get");
		assert!(got.is_some());
		assert_eq!(got.unwrap(), data);

		let _ = fs::remove_file(&fname);
	}

	#[test]
	fn test_securedb_blob_encrypted_in_fallback() {
		let tmp = env::temp_dir();
		let fname = tmp.join(format!("passman_test_{}.db", Uuid::new_v4()));
		let master = "test_master_password";
		let _ = fs::remove_file(&fname);

		let db = SecureDb::open(&fname, master).expect("open db");
		let key = "service2";
		let data = b"another secret";
		db.put(key, data, master).expect("put");

		// Directly query stored blob to ensure it's encrypted (not equal to plaintext)
		let conn = &db.conn;
		let blob_opt: Option<Vec<u8>> = conn.query_row(
			"SELECT v FROM encrypted_blob WHERE k = ?1", params![key], |r| r.get(0)
		).optional().expect("query");

		assert!(blob_opt.is_some());
		let blob = blob_opt.unwrap();
		assert!(blob.len() > data.len());
		assert_ne!(&blob[32..], data);

		let _ = fs::remove_file(&fname);
	}
}
