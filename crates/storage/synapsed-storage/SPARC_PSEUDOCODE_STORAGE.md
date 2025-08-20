# SPARC Pseudocode Phase - Storage Backend Algorithms

## Overview
This document contains pseudocode designs for the core storage algorithms and data structures in synapsed-storage.

## 1. Core Storage Backend Interface

```pseudocode
INTERFACE StorageBackend:
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result
    FUNCTION delete(key: ByteArray) -> Result<Boolean>
    FUNCTION scan(prefix: ByteArray) -> Iterator<(ByteArray, ByteArray)>
    FUNCTION batch_write(operations: List<Operation>) -> Result
    FUNCTION sync() -> Result
    FUNCTION stats() -> StorageStats
```

## 2. Memory Backend Algorithm

```pseudocode
CLASS MemoryStorage IMPLEMENTS StorageBackend:
    PRIVATE data: BTreeMap<ByteArray, ByteArray>
    PRIVATE lock: RwLock
    PRIVATE stats: StorageStats
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        lock.read_lock()
        stats.read_count += 1
        result = data.get(key)
        lock.read_unlock()
        RETURN result.clone()
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        lock.write_lock()
        old_value = data.get(key)
        IF old_value EXISTS:
            stats.total_bytes -= old_value.length
        ELSE:
            stats.entry_count += 1
        
        data.insert(key, value)
        stats.total_bytes += value.length
        stats.write_count += 1
        lock.write_unlock()
        RETURN Success
    
    FUNCTION delete(key: ByteArray) -> Result<Boolean>:
        lock.write_lock()
        old_value = data.remove(key)
        IF old_value EXISTS:
            stats.total_bytes -= old_value.length
            stats.entry_count -= 1
            stats.delete_count += 1
            result = True
        ELSE:
            result = False
        lock.write_unlock()
        RETURN Success(result)
    
    FUNCTION scan(prefix: ByteArray) -> Iterator:
        lock.read_lock()
        // Create snapshot iterator
        entries = []
        FOR (key, value) IN data.range(prefix..):
            IF NOT key.starts_with(prefix):
                BREAK
            entries.append((key.clone(), value.clone()))
        lock.read_unlock()
        RETURN entries.into_iterator()
```

## 3. SQLite Backend Algorithm

```pseudocode
CLASS SqliteStorage IMPLEMENTS StorageBackend:
    PRIVATE connection: SqliteConnection
    PRIVATE prepared_statements: HashMap<String, PreparedStatement>
    
    FUNCTION initialize(path: String) -> Result:
        connection = SqliteConnection.open(path)
        
        // Create table with optimized schema
        connection.execute("
            CREATE TABLE IF NOT EXISTS storage (
                key BLOB PRIMARY KEY,
                value BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                compressed BOOLEAN DEFAULT FALSE,
                checksum BLOB
            ) WITHOUT ROWID
        ")
        
        // Create index for prefix scans
        connection.execute("
            CREATE INDEX IF NOT EXISTS idx_key_prefix 
            ON storage(key)
        ")
        
        // Prepare statements for performance
        prepared_statements["get"] = connection.prepare(
            "SELECT value, compressed FROM storage WHERE key = ?"
        )
        prepared_statements["put"] = connection.prepare(
            "INSERT OR REPLACE INTO storage 
             (key, value, created_at, updated_at, compressed, checksum) 
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        prepared_statements["delete"] = connection.prepare(
            "DELETE FROM storage WHERE key = ?"
        )
        
        RETURN Success
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        stmt = prepared_statements["get"]
        row = stmt.query_row(key)
        
        IF row EXISTS:
            value = row.value
            IF row.compressed:
                value = decompress(value)
            RETURN Some(value)
        ELSE:
            RETURN None
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        compressed = FALSE
        stored_value = value
        
        // Compress if beneficial
        IF value.length > COMPRESSION_THRESHOLD:
            compressed_value = compress(value)
            IF compressed_value.length < value.length * 0.8:
                stored_value = compressed_value
                compressed = TRUE
        
        checksum = calculate_checksum(stored_value)
        timestamp = current_timestamp()
        
        stmt = prepared_statements["put"]
        stmt.execute(key, stored_value, timestamp, timestamp, compressed, checksum)
        
        RETURN Success
    
    FUNCTION batch_write(operations: List<Operation>) -> Result:
        transaction = connection.begin_transaction()
        
        TRY:
            FOR op IN operations:
                MATCH op:
                    Put(key, value) -> self.put(key, value)
                    Delete(key) -> self.delete(key)
            
            transaction.commit()
            RETURN Success
        CATCH error:
            transaction.rollback()
            RETURN Error(error)
```

## 4. RocksDB Backend Algorithm

```pseudocode
CLASS RocksDbStorage IMPLEMENTS StorageBackend:
    PRIVATE db: RocksDB
    PRIVATE write_options: WriteOptions
    PRIVATE read_options: ReadOptions
    
    FUNCTION initialize(path: String, config: RocksDbConfig) -> Result:
        options = RocksDbOptions.new()
        
        // Optimize for SSD
        options.set_compression_type(CompressionType.LZ4)
        options.set_block_cache_size(config.cache_size)
        options.set_write_buffer_size(config.write_buffer_size)
        options.set_max_open_files(config.max_open_files)
        
        // Enable bloom filters for faster lookups
        block_options = BlockBasedOptions.new()
        block_options.set_bloom_filter(10)
        options.set_block_based_table_factory(block_options)
        
        db = RocksDB.open(options, path)?
        
        write_options = WriteOptions.new()
        write_options.set_sync(FALSE)  // Async writes by default
        
        read_options = ReadOptions.new()
        
        RETURN Success
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        result = db.get_cf_opt(read_options, key)
        RETURN result
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        db.put_cf_opt(write_options, key, value)
        RETURN Success
    
    FUNCTION delete(key: ByteArray) -> Result<Boolean>:
        // Check existence first
        exists = db.get_cf_opt(read_options, key).is_some()
        IF exists:
            db.delete_cf_opt(write_options, key)
        RETURN Success(exists)
    
    FUNCTION batch_write(operations: List<Operation>) -> Result:
        batch = WriteBatch.new()
        
        FOR op IN operations:
            MATCH op:
                Put(key, value) -> batch.put(key, value)
                Delete(key) -> batch.delete(key)
        
        db.write_opt(batch, write_options)
        RETURN Success
```

## 5. Cache Layer Algorithm (LRU)

```pseudocode
CLASS LruCache:
    PRIVATE capacity: Integer
    PRIVATE cache: LinkedHashMap<ByteArray, CacheEntry>
    PRIVATE lock: RwLock
    PRIVATE stats: CacheStats
    
    STRUCTURE CacheEntry:
        value: ByteArray
        size: Integer
        timestamp: Integer
        access_count: Integer
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        lock.write_lock()  // Need write lock for LRU update
        
        entry = cache.get(key)
        IF entry EXISTS:
            // Move to front (most recently used)
            cache.remove(key)
            cache.insert(key, entry)
            
            entry.access_count += 1
            entry.timestamp = current_timestamp()
            stats.hits += 1
            
            result = Some(entry.value.clone())
        ELSE:
            stats.misses += 1
            result = None
        
        lock.write_unlock()
        RETURN result
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        lock.write_lock()
        
        entry = CacheEntry {
            value: value,
            size: value.length,
            timestamp: current_timestamp(),
            access_count: 0
        }
        
        // Remove old entry if exists
        IF cache.contains(key):
            old_entry = cache.remove(key)
            stats.total_size -= old_entry.size
        
        // Evict entries if needed
        WHILE stats.total_size + entry.size > capacity AND NOT cache.is_empty():
            // Remove least recently used (first in LinkedHashMap)
            (evict_key, evict_entry) = cache.pop_front()
            stats.total_size -= evict_entry.size
            stats.evictions += 1
        
        // Insert new entry
        cache.insert(key, entry)
        stats.total_size += entry.size
        
        lock.write_unlock()
        RETURN Success
```

## 6. Compression Layer Algorithm

```pseudocode
CLASS CompressionLayer:
    PRIVATE backend: StorageBackend
    PRIVATE algorithm: CompressionAlgorithm
    PRIVATE min_size: Integer
    PRIVATE compression_level: Integer
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        metadata = CompressionMetadata {
            original_size: value.length,
            compressed: FALSE,
            algorithm: None
        }
        
        stored_value = value
        
        // Only compress if above threshold
        IF value.length >= min_size:
            compressed = compress_with_algorithm(value, algorithm, compression_level)
            
            // Only use compression if it saves space
            compression_ratio = compressed.length / value.length
            IF compression_ratio < 0.9:  // 10% savings threshold
                stored_value = compressed
                metadata.compressed = TRUE
                metadata.algorithm = algorithm
        
        // Store with metadata prefix
        final_value = serialize_metadata(metadata) + stored_value
        RETURN backend.put(key, final_value)
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        stored_value = backend.get(key)?
        
        // Extract metadata
        (metadata, data) = deserialize_metadata(stored_value)
        
        IF metadata.compressed:
            data = decompress_with_algorithm(data, metadata.algorithm)
        
        RETURN Some(data)
    
    FUNCTION compress_with_algorithm(data: ByteArray, algo: Algorithm, level: Integer) -> ByteArray:
        MATCH algo:
            LZ4 -> lz4_compress(data, level)
            ZSTD -> zstd_compress(data, level)
            SNAPPY -> snappy_compress(data)
```

## 7. Encryption Layer Algorithm

```pseudocode
CLASS EncryptionLayer:
    PRIVATE backend: StorageBackend
    PRIVATE key_manager: KeyManager
    PRIVATE algorithm: EncryptionAlgorithm
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        // Generate key ID for key rotation support
        key_id = key_manager.current_key_id()
        encryption_key = key_manager.get_key(key_id)
        
        // Generate nonce for this value
        nonce = generate_random_nonce()
        
        // Encrypt with authenticated encryption
        ciphertext = encrypt_aead(
            algorithm,
            encryption_key,
            nonce,
            value,
            key  // Use storage key as additional authenticated data
        )
        
        // Create envelope with metadata
        envelope = EncryptionEnvelope {
            version: 1,
            key_id: key_id,
            algorithm: algorithm,
            nonce: nonce,
            ciphertext: ciphertext
        }
        
        serialized = serialize_envelope(envelope)
        RETURN backend.put(key, serialized)
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        serialized = backend.get(key)?
        envelope = deserialize_envelope(serialized)
        
        // Get appropriate key for decryption
        decryption_key = key_manager.get_key(envelope.key_id)
        
        // Decrypt with authentication
        plaintext = decrypt_aead(
            envelope.algorithm,
            decryption_key,
            envelope.nonce,
            envelope.ciphertext,
            key  // Additional authenticated data
        )
        
        RETURN Some(plaintext)
    
    FUNCTION rotate_keys() -> Result:
        new_key_id = key_manager.generate_new_key()
        
        // Re-encrypt all data with new key
        iterator = backend.scan(EMPTY_PREFIX)
        batch_operations = []
        
        FOR (storage_key, encrypted_value) IN iterator:
            // Decrypt with old key
            plaintext = self.get(storage_key)?
            
            // Re-encrypt with new key
            key_manager.set_current_key_id(new_key_id)
            new_envelope = self.create_encrypted_envelope(storage_key, plaintext)
            
            batch_operations.append(Put(storage_key, new_envelope))
        
        // Apply all re-encryptions atomically
        backend.batch_write(batch_operations)
        key_manager.mark_rotation_complete(new_key_id)
        
        RETURN Success
```

## 8. CRDT Synchronization Algorithm

```pseudocode
CLASS CrdtSyncEngine:
    PRIVATE storage: StorageBackend
    PRIVATE vector_clock: VectorClock
    PRIVATE peer_id: PeerId
    
    STRUCTURE CrdtEntry:
        value: ByteArray
        clock: VectorClock
        tombstone: Boolean
        last_writer: PeerId
    
    FUNCTION merge_remote_entry(key: ByteArray, remote_entry: CrdtEntry) -> Result:
        local_entry = storage.get_crdt_entry(key)
        
        IF local_entry NOT EXISTS:
            // Simple case: new entry
            storage.put_crdt_entry(key, remote_entry)
            RETURN Success
        
        // Compare vector clocks
        comparison = compare_vector_clocks(local_entry.clock, remote_entry.clock)
        
        MATCH comparison:
            BEFORE ->
                // Remote is newer, take it
                storage.put_crdt_entry(key, remote_entry)
            
            AFTER ->
                // Local is newer, keep it
                RETURN Success
            
            CONCURRENT ->
                // Conflict! Use deterministic resolution
                merged_entry = resolve_conflict(local_entry, remote_entry)
                storage.put_crdt_entry(key, merged_entry)
        
        RETURN Success
    
    FUNCTION resolve_conflict(local: CrdtEntry, remote: CrdtEntry) -> CrdtEntry:
        // Last-writer-wins with peer ID as tiebreaker
        IF local.clock.timestamp > remote.clock.timestamp:
            RETURN local
        ELSE IF remote.clock.timestamp > local.clock.timestamp:
            RETURN remote
        ELSE:
            // Same timestamp, use peer ID for deterministic order
            IF local.last_writer > remote.last_writer:
                RETURN local
            ELSE:
                RETURN remote
    
    FUNCTION sync_with_peer(peer: Peer) -> Result:
        // Get sync state
        local_state = get_sync_state()
        remote_state = peer.get_sync_state()
        
        // Compute differences
        need_from_peer = compute_missing_entries(local_state, remote_state)
        need_to_send = compute_missing_entries(remote_state, local_state)
        
        // Exchange data
        remote_entries = peer.get_entries(need_from_peer)
        FOR entry IN remote_entries:
            merge_remote_entry(entry.key, entry)
        
        local_entries = storage.get_entries(need_to_send)
        peer.receive_entries(local_entries)
        
        RETURN Success
```

## Next Steps

1. **Validate Algorithms**: Review each algorithm for correctness and efficiency
2. **Performance Analysis**: Identify potential bottlenecks and optimization opportunities
3. **Error Handling**: Add comprehensive error handling to each algorithm
4. **Concurrency**: Ensure thread-safety and optimize lock usage
5. **Memory Management**: Plan for efficient memory usage and garbage collection

---

**Document Status**: Initial pseudocode design complete
**Next Phase**: Architecture - Define module boundaries and implementation structure