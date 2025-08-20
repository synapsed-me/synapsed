# SPARC Pseudocode Phase - Caching Algorithms

## Overview
This document contains pseudocode designs for various caching strategies in synapsed-storage, including LRU, LFU, ARC, and distributed caching.

## 1. Cache Interface

```pseudocode
INTERFACE Cache:
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result
    FUNCTION delete(key: ByteArray) -> Result
    FUNCTION clear() -> Result
    FUNCTION stats() -> CacheStats
    FUNCTION resize(new_capacity: Integer) -> Result
```

## 2. LRU (Least Recently Used) Cache

```pseudocode
CLASS LruCache IMPLEMENTS Cache:
    PRIVATE capacity: Integer
    PRIVATE current_size: Integer
    PRIVATE cache_map: HashMap<ByteArray, Node>
    PRIVATE head: Node  // Most recently used
    PRIVATE tail: Node  // Least recently used
    PRIVATE lock: RwLock
    PRIVATE stats: CacheStats
    
    STRUCTURE Node:
        key: ByteArray
        value: ByteArray
        size: Integer
        prev: Optional<Node>
        next: Optional<Node>
        timestamp: Integer
    
    FUNCTION initialize(capacity: Integer):
        self.capacity = capacity
        self.current_size = 0
        self.cache_map = HashMap.new()
        
        // Initialize dummy head and tail
        self.head = Node.dummy()
        self.tail = Node.dummy()
        self.head.next = self.tail
        self.tail.prev = self.head
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        lock.read_lock()
        node = cache_map.get(key)
        
        IF node EXISTS:
            lock.read_unlock()
            lock.write_lock()
            
            // Move to head (most recently used)
            remove_node(node)
            add_to_head(node)
            
            stats.hits += 1
            node.timestamp = current_timestamp()
            
            result = Some(node.value.clone())
            lock.write_unlock()
        ELSE:
            stats.misses += 1
            result = None
            lock.read_unlock()
        
        RETURN result
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        lock.write_lock()
        
        new_size = value.length
        
        // Check if key already exists
        IF cache_map.contains(key):
            node = cache_map.get(key)
            current_size -= node.size
            remove_node(node)
        
        // Evict entries if needed
        WHILE current_size + new_size > capacity AND NOT is_empty():
            evict_lru()
        
        // Create and add new node
        new_node = Node {
            key: key.clone(),
            value: value,
            size: new_size,
            timestamp: current_timestamp()
        }
        
        add_to_head(new_node)
        cache_map.insert(key, new_node)
        current_size += new_size
        
        lock.write_unlock()
        RETURN Success
    
    PRIVATE FUNCTION evict_lru():
        lru_node = tail.prev
        IF lru_node != head:
            remove_node(lru_node)
            cache_map.remove(lru_node.key)
            current_size -= lru_node.size
            stats.evictions += 1
    
    PRIVATE FUNCTION remove_node(node: Node):
        node.prev.next = node.next
        node.next.prev = node.prev
    
    PRIVATE FUNCTION add_to_head(node: Node):
        node.prev = head
        node.next = head.next
        head.next.prev = node
        head.next = node
```

## 3. LFU (Least Frequently Used) Cache

```pseudocode
CLASS LfuCache IMPLEMENTS Cache:
    PRIVATE capacity: Integer
    PRIVATE current_size: Integer
    PRIVATE min_frequency: Integer
    PRIVATE cache_map: HashMap<ByteArray, Node>
    PRIVATE frequency_map: HashMap<Integer, LinkedList<Node>>
    PRIVATE lock: RwLock
    PRIVATE stats: CacheStats
    
    STRUCTURE Node:
        key: ByteArray
        value: ByteArray
        size: Integer
        frequency: Integer
        timestamp: Integer
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        lock.write_lock()
        
        node = cache_map.get(key)
        IF node EXISTS:
            update_frequency(node)
            stats.hits += 1
            result = Some(node.value.clone())
        ELSE:
            stats.misses += 1
            result = None
        
        lock.write_unlock()
        RETURN result
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        lock.write_lock()
        
        new_size = value.length
        
        // Update existing entry
        IF cache_map.contains(key):
            node = cache_map.get(key)
            current_size -= node.size
            node.value = value
            node.size = new_size
            current_size += new_size
            update_frequency(node)
        ELSE:
            // Evict if needed
            WHILE current_size + new_size > capacity AND NOT is_empty():
                evict_lfu()
            
            // Add new entry
            new_node = Node {
                key: key.clone(),
                value: value,
                size: new_size,
                frequency: 1,
                timestamp: current_timestamp()
            }
            
            cache_map.insert(key, new_node)
            add_to_frequency_list(new_node, 1)
            current_size += new_size
            min_frequency = 1
        
        lock.write_unlock()
        RETURN Success
    
    PRIVATE FUNCTION update_frequency(node: Node):
        old_freq = node.frequency
        new_freq = old_freq + 1
        
        // Remove from old frequency list
        freq_list = frequency_map.get(old_freq)
        freq_list.remove(node)
        
        IF freq_list.is_empty():
            frequency_map.remove(old_freq)
            IF min_frequency == old_freq:
                min_frequency += 1
        
        // Add to new frequency list
        node.frequency = new_freq
        add_to_frequency_list(node, new_freq)
    
    PRIVATE FUNCTION evict_lfu():
        // Get least frequently used list
        lfu_list = frequency_map.get(min_frequency)
        
        IF lfu_list EXISTS AND NOT lfu_list.is_empty():
            // Within same frequency, use LRU (remove from head)
            lfu_node = lfu_list.remove_first()
            cache_map.remove(lfu_node.key)
            current_size -= lfu_node.size
            stats.evictions += 1
```

## 4. ARC (Adaptive Replacement Cache)

```pseudocode
CLASS ArcCache IMPLEMENTS Cache:
    PRIVATE capacity: Integer
    PRIVATE p: Integer  // Target size for T1
    
    // Four lists: T1, T2 (cached), B1, B2 (ghost - metadata only)
    PRIVATE t1: LruList  // Recent cache entries
    PRIVATE t2: LruList  // Frequent cache entries
    PRIVATE b1: LruList  // Recent evictions (ghost)
    PRIVATE b2: LruList  // Frequent evictions (ghost)
    
    PRIVATE cache_map: HashMap<ByteArray, CacheLocation>
    PRIVATE lock: RwLock
    PRIVATE stats: CacheStats
    
    ENUM CacheLocation:
        T1, T2, B1, B2
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        lock.write_lock()
        
        location = cache_map.get(key)
        
        MATCH location:
            Some(T1) ->
                // Move from T1 to T2 (promote to frequent)
                node = t1.remove(key)
                t2.add(node)
                cache_map.update(key, T2)
                stats.hits += 1
                result = Some(node.value.clone())
            
            Some(T2) ->
                // Already frequent, just update position
                node = t2.get(key)
                t2.move_to_mru(node)
                stats.hits += 1
                result = Some(node.value.clone())
            
            Some(B1) ->
                // Cache miss, but in recent ghost list
                adapt_p(delta = 1)
                replace(key)
                stats.misses += 1
                result = None
            
            Some(B2) ->
                // Cache miss, but in frequent ghost list
                adapt_p(delta = -1)
                replace(key)
                stats.misses += 1
                result = None
            
            None ->
                // Complete miss
                stats.misses += 1
                result = None
        
        lock.write_unlock()
        RETURN result
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        lock.write_lock()
        
        location = cache_map.get(key)
        
        MATCH location:
            Some(T1) | Some(T2) ->
                // Update existing entry
                update_existing(key, value, location)
            
            Some(B1) ->
                // Was recently evicted, bring back
                adapt_p(delta = 1)
                b1.remove(key)
                add_to_cache(key, value, T2)
            
            Some(B2) ->
                // Was frequently accessed before eviction
                adapt_p(delta = -1)
                b2.remove(key)
                add_to_cache(key, value, T2)
            
            None ->
                // New entry
                IF t1.size() + b1.size() >= capacity:
                    // Case A: T1 ∪ B1 is full
                    IF t1.size() < capacity:
                        // Room in cache
                        b1.evict_lru()
                        replace(key)
                    ELSE:
                        // Cache full, evict from T1
                        evict_from_t1()
                ELSE:
                    // Case B: T1 ∪ B1 has space
                    total_ghost = b1.size() + b2.size()
                    IF total_ghost >= capacity:
                        // Ghost lists full
                        IF total_ghost == 2 * capacity:
                            b2.evict_lru()
                        replace(key)
                
                add_to_cache(key, value, T1)
        
        lock.write_unlock()
        RETURN Success
    
    PRIVATE FUNCTION adapt_p(delta: Integer):
        // Adapt the target size for T1
        p = max(0, min(capacity, p + delta))
    
    PRIVATE FUNCTION replace(key: ByteArray):
        IF t1.size() >= max(1, p):
            // T1 is too big, evict from T1
            evict_from_t1()
        ELSE:
            // T2 is too big, evict from T2
            evict_from_t2()
    
    PRIVATE FUNCTION evict_from_t1():
        victim = t1.evict_lru()
        b1.add_ghost(victim.key)
        cache_map.update(victim.key, B1)
        stats.evictions += 1
    
    PRIVATE FUNCTION evict_from_t2():
        victim = t2.evict_lru()
        b2.add_ghost(victim.key)
        cache_map.update(victim.key, B2)
        stats.evictions += 1
```

## 5. Write-Through vs Write-Back Cache

```pseudocode
CLASS WritePolicy:
    ENUM Type:
        WRITE_THROUGH,  // Write to cache and backend immediately
        WRITE_BACK,     // Write to cache, flush to backend later
        WRITE_AROUND    // Write to backend only, invalidate cache
    
CLASS WriteThroughCache IMPLEMENTS Cache:
    PRIVATE cache: Cache
    PRIVATE backend: StorageBackend
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        // Write to backend first
        backend.put(key, value)?
        
        // Then update cache
        cache.put(key, value)?
        
        RETURN Success

CLASS WriteBackCache IMPLEMENTS Cache:
    PRIVATE cache: Cache
    PRIVATE backend: StorageBackend
    PRIVATE dirty_set: HashSet<ByteArray>
    PRIVATE flush_interval: Duration
    PRIVATE last_flush: Timestamp
    PRIVATE lock: RwLock
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        lock.write_lock()
        
        // Write to cache only
        cache.put(key, value)?
        
        // Mark as dirty
        dirty_set.insert(key)
        
        // Check if we should flush
        IF should_flush():
            flush_dirty_entries()
        
        lock.write_unlock()
        RETURN Success
    
    FUNCTION flush_dirty_entries() -> Result:
        batch = WriteBatch.new()
        
        FOR key IN dirty_set:
            value = cache.get(key)
            IF value EXISTS:
                batch.put(key, value)
        
        // Write all dirty entries to backend
        backend.batch_write(batch)?
        
        // Clear dirty set
        dirty_set.clear()
        last_flush = current_timestamp()
        
        RETURN Success
    
    PRIVATE FUNCTION should_flush() -> Boolean:
        RETURN dirty_set.size() > MAX_DIRTY_ENTRIES OR
               current_timestamp() - last_flush > flush_interval
```

## 6. Distributed Cache

```pseudocode
CLASS DistributedCache IMPLEMENTS Cache:
    PRIVATE local_cache: Cache
    PRIVATE peer_nodes: List<CacheNode>
    PRIVATE consistent_hash: ConsistentHash
    PRIVATE replication_factor: Integer
    
    FUNCTION get(key: ByteArray) -> Optional<ByteArray>:
        // Check local cache first
        value = local_cache.get(key)
        IF value EXISTS:
            RETURN value
        
        // Find responsible nodes
        nodes = consistent_hash.get_nodes(key, replication_factor)
        
        // Try each node in order
        FOR node IN nodes:
            IF node == self:
                CONTINUE
            
            TRY:
                value = node.remote_get(key)
                IF value EXISTS:
                    // Cache locally for future reads
                    local_cache.put(key, value)
                    RETURN Some(value)
            CATCH NetworkError:
                CONTINUE
        
        RETURN None
    
    FUNCTION put(key: ByteArray, value: ByteArray) -> Result:
        // Find responsible nodes
        nodes = consistent_hash.get_nodes(key, replication_factor)
        
        success_count = 0
        errors = []
        
        // Write to all replica nodes
        PARALLEL FOR node IN nodes:
            TRY:
                IF node == self:
                    local_cache.put(key, value)
                ELSE:
                    node.remote_put(key, value)
                success_count += 1
            CATCH error:
                errors.append(error)
        
        // Require quorum for success
        quorum = (replication_factor / 2) + 1
        IF success_count >= quorum:
            RETURN Success
        ELSE:
            RETURN Error("Failed to achieve write quorum")
    
    FUNCTION handle_node_failure(failed_node: CacheNode):
        // Re-hash keys that were on failed node
        affected_keys = estimate_affected_keys(failed_node)
        
        FOR key IN affected_keys:
            new_nodes = consistent_hash.get_nodes(key, replication_factor)
            
            // Ensure data is replicated to new nodes
            value = self.get(key)
            IF value EXISTS:
                FOR node IN new_nodes:
                    IF node != self AND node != failed_node:
                        node.remote_put(key, value)
```

## 7. Cache Warming and Preloading

```pseudocode
CLASS CacheWarmer:
    PRIVATE cache: Cache
    PRIVATE backend: StorageBackend
    PRIVATE predictor: AccessPredictor
    
    FUNCTION warm_cache(patterns: List<AccessPattern>) -> Result:
        predicted_keys = predictor.predict_next_accesses(patterns)
        
        // Sort by predicted access probability
        predicted_keys.sort_by(|k| k.probability, DESCENDING)
        
        // Load until cache is 80% full
        target_size = cache.capacity * 0.8
        current_size = 0
        
        FOR key_prediction IN predicted_keys:
            IF current_size >= target_size:
                BREAK
            
            value = backend.get(key_prediction.key)
            IF value EXISTS:
                cache.put(key_prediction.key, value)
                current_size += value.length
        
        RETURN Success
    
    FUNCTION adaptive_preload(access_log: AccessLog) -> Result:
        // Analyze access patterns
        patterns = analyze_patterns(access_log)
        
        FOR pattern IN patterns:
            MATCH pattern:
                Sequential(prefix, stride) ->
                    preload_sequential(prefix, stride)
                
                Temporal(time_pattern) ->
                    schedule_temporal_preload(time_pattern)
                
                Correlated(key_groups) ->
                    preload_correlated(key_groups)
        
        RETURN Success
```

## Performance Considerations

1. **Lock Granularity**: Use fine-grained locking or lock-free structures where possible
2. **Memory Efficiency**: Track memory usage accurately, including metadata overhead
3. **Eviction Performance**: Ensure O(1) eviction for all cache types
4. **Hit Rate Optimization**: Monitor and adapt cache parameters based on workload
5. **Network Efficiency**: Batch operations in distributed cache scenarios

## Next Steps

1. **Benchmark Different Algorithms**: Compare LRU, LFU, and ARC for various workloads
2. **Implement Adaptive Sizing**: Dynamic cache size based on memory pressure
3. **Add Metrics Collection**: Detailed statistics for cache performance analysis
4. **Design TTL Support**: Time-based expiration for cache entries
5. **Plan Persistence**: Optional cache persistence for warm restarts

---

**Document Status**: Cache algorithm pseudocode complete
**Next**: Design compression and encryption layer algorithms