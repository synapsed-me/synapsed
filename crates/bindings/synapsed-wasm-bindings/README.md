# Synapsed WASM Bindings

WebAssembly bindings for the Synapsed framework, enabling verifiable AI agent systems to run in browsers and other WASM environments.

## Features

- **Intent Management**: Create and verify hierarchical intent trees
- **Promise Theory**: Implement cooperation protocols between agents
- **Verification Strategies**: Multiple verification methods for agent claims
- **Observability**: Event emission and metrics collection using Substrates patterns

## Installation

### For Browser (using a bundler like webpack)
```bash
npm install @synapsed/wasm
```

### For Browser (direct usage)
```html
<script type="module">
import init, { WasmIntent, WasmPromise } from './synapsed_wasm.js';

async function run() {
    await init();
    
    const intent = new WasmIntent("Complete user task");
    console.log("Intent ID:", intent.id);
}

run();
</script>
```

### For Node.js
```javascript
const { WasmIntent, WasmPromise } = require('@synapsed/wasm');

const intent = new WasmIntent("Complete user task");
console.log("Intent ID:", intent.id);
```

## Quick Start

### Creating an Intent
```javascript
import { WasmIntent, WasmIntentBuilder } from '@synapsed/wasm';

// Simple intent
const intent = new WasmIntent("Process data");

// Using builder pattern
const complexIntent = new WasmIntentBuilder("Complex task")
    .with_description("A complex multi-step task")
    .with_priority("high")
    .build();

// Add steps
intent.add_step("fetch_data", "Fetch data from API");
intent.add_step("process", "Process the data");
intent.add_step("store", "Store results");
```

### Creating Promises
```javascript
import { WasmPromise, WasmTrustModel } from '@synapsed/wasm';

// Create a promise between agents
const promise = new WasmPromise(
    "Deliver processed data within 5 seconds",
    "agent1",  // promiser
    "agent2"   // promisee
);

// Accept and fulfill
promise.accept();  // Agent2 accepts
promise.fulfill(); // Agent1 fulfills

// Trust model
const trust = new WasmTrustModel();
trust.add_agent("agent1", 0.8);
trust.update_trust("agent1", true); // Fulfilled promise increases trust
```

### Verification
```javascript
import { WasmCommandVerifier, WasmFileSystemVerifier, WasmCompositeVerifier } from '@synapsed/wasm';

// Command verification
const cmdVerifier = new WasmCommandVerifier();
const result1 = cmdVerifier.verify_command("ls -la");
console.log("Verified:", result1.verified);

// File system verification
const fsVerifier = new WasmFileSystemVerifier();
fsVerifier.expect_file("/output/result.json");
const result2 = fsVerifier.verify_files(["/output/result.json"]);

// Composite verification
const composite = new WasmCompositeVerifier();
composite.add_result(result1);
composite.add_result(result2);
const overall = composite.get_overall_result();
```

### Observability
```javascript
import { WasmObservable, WasmEventEmitter, WasmMetricsCollector } from '@synapsed/wasm';

// Create observable wrapper
const observable = new WasmObservable("my-operation");

// Start traced operation
observable.start_operation("data_processing");

// Do work...

// End operation (automatically records metrics)
observable.end_operation();

// Get summary
const summary = observable.get_summary();
console.log("Summary:", summary);

// Direct event emission
const emitter = new WasmEventEmitter("my-circuit");
emitter.emit(new WasmEvent("custom_event", "data", "source"));

// Add listener
emitter.add_listener((event) => {
    console.log("Event received:", event);
});
```

## Building from Source

1. Install Rust and wasm-pack:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

2. Build the package:
```bash
./build.sh
```

This creates packages for:
- `pkg/bundler/` - For webpack/rollup
- `pkg/web/` - For direct browser usage  
- `pkg/node/` - For Node.js

## API Documentation

Full API documentation is available at [https://docs.synapsed.me/wasm](https://docs.synapsed.me/wasm)

## Examples

See the `examples/wasm-browser/` directory for complete examples.

## License

MIT OR Apache-2.0