# WASM Browser Example

This example demonstrates running the Synapsed framework directly in web browsers using WebAssembly, enabling client-side intent verification and agent cooperation.

## Overview

The WASM browser example showcases:
- Intent management in the browser
- Promise Theory implementation
- Trust model with reputation tracking
- Multiple verification strategies
- Real-time observability metrics

## Setup

1. Build the WASM package:
```bash
cd ../../crates/bindings/synapsed-wasm-bindings
./build.sh
```

2. Copy the generated package:
```bash
cp -r pkg ../../examples/wasm-browser/
```

3. Serve the example:
```bash
cd ../../examples/wasm-browser
python3 -m http.server 8000
# or
npx serve .
```

4. Open in browser:
```
http://localhost:8000
```

## Features

### Intent Management
- Create intents with goals and priorities
- Execute multi-step intents
- Verify intent completion
- Track intent status

### Promise Cooperation
- Create promises between agents
- Accept/reject promises
- Fulfill or violate promises
- Automatic trust updates

### Trust Model
- Add agents with initial trust scores
- Update trust based on promise fulfillment
- Query trust scores and trustworthiness
- Threshold-based decision making

### Verification Strategies
- Command verification (safety checks)
- File system verification
- Composite verification (multiple strategies)
- Confidence scoring

### Observability
- Real-time event emission
- Metrics collection
- Operation tracking
- Performance monitoring

## Architecture

```
┌──────────────┐
│   Browser    │
│              │
│  ┌────────┐  │
│  │  WASM  │  │     JavaScript API
│  │ Module │  ├──────────────────────►
│  └────────┘  │
│              │     ┌──────────────┐
│  JavaScript  │────►│   Synapsed   │
│   Bindings   │     │   Framework  │
│              │     └──────────────┘
└──────────────┘            │
                            ▼
                   ┌─────────────────┐
                   │  Intent System  │
                   │  Trust Model    │
                   │  Verification   │
                   │  Observability  │
                   └─────────────────┘
```

## JavaScript API

### Intent Management
```javascript
// Create intent
const builder = new WasmIntentBuilder("Process data");
const intent = builder
    .with_description("Browser task")
    .with_priority("high")
    .build();

// Add steps
intent.add_step("fetch", "Fetch data");
intent.add_step("process", "Process data");

// Verify
const result = await verify_intent(intent.id);
```

### Promise Theory
```javascript
// Create promise
const promise = new WasmPromise(
    "Complete in 5s",
    "agent1",
    "agent2"
);

// Lifecycle
promise.accept();
promise.fulfill();
```

### Trust Model
```javascript
const trust = new WasmTrustModel();
trust.add_agent("agent1", 0.7);
trust.update_trust("agent1", true); // fulfilled
const score = trust.get_trust("agent1");
```

### Verification
```javascript
// Command verification
const cmdVerifier = new WasmCommandVerifier();
const result = cmdVerifier.verify_command("ls -la");

// Composite verification
const composite = new WasmCompositeVerifier();
composite.add_result(result1);
composite.add_result(result2);
const overall = composite.get_overall_result();
```

### Observability
```javascript
// Create observable
const observable = new WasmObservable("my-app");
observable.start_operation("user_action");

// Emit events
const emitter = new WasmEventEmitter("circuit");
emitter.emit(new WasmEvent("click", data, "ui"));

// Collect metrics
const metrics = new WasmMetricsCollector();
metrics.record_count("clicks", 1);
```

## Performance

The WASM module is optimized for size and speed:
- Module size: ~200KB gzipped
- Initialization: <100ms
- Operation overhead: <1ms
- Memory usage: <10MB typical

## Security

- All verification happens client-side
- No sensitive data sent to servers
- Cryptographic proofs generated locally
- Trust scores stored in browser storage

## Browser Compatibility

- Chrome 89+
- Firefox 89+
- Safari 15+
- Edge 89+

Requires:
- WebAssembly support
- ES6 modules
- Async/await

## Development

To modify the WASM bindings:

1. Edit the Rust code in `crates/bindings/synapsed-wasm-bindings/`
2. Rebuild: `wasm-pack build --target web`
3. Copy new package: `cp -r pkg ../../examples/wasm-browser/`
4. Refresh browser

## Related Examples

- `intent-verification`: Server-side verification
- `mcp-server`: Integration with Claude
- `promise-cooperation`: Full Promise Theory implementation