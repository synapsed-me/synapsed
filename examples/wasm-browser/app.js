// Import the WASM module
import init, {
    WasmIntent,
    WasmIntentBuilder,
    WasmPromise,
    WasmTrustModel,
    WasmCommandVerifier,
    WasmFileSystemVerifier,
    WasmCompositeVerifier,
    WasmObservable,
    WasmEventEmitter,
    WasmMetricsCollector,
    WasmEvent,
    verify_intent,
    intent_from_json
} from './pkg/synapsed_wasm.js';

// Global variables
let wasmModule = null;
let currentIntent = null;
let currentPromise = null;
let trustModel = null;
let observable = null;
let eventEmitter = null;
let metricsCollector = null;

// Initialize WASM module on page load
async function initializeWasm() {
    try {
        showLoader(true);
        wasmModule = await init();
        
        // Initialize components
        trustModel = new WasmTrustModel();
        observable = new WasmObservable("browser-demo");
        eventEmitter = new WasmEventEmitter("demo-circuit");
        metricsCollector = new WasmMetricsCollector();
        
        log('intentOutput', '✅ WASM module initialized successfully');
        log('promiseOutput', '✅ Promise system ready');
        log('trustOutput', '✅ Trust model initialized');
        log('verificationOutput', '✅ Verification strategies loaded');
        log('observabilityOutput', '✅ Observability system online');
        
        // Add event listener
        eventEmitter.add_listener((event) => {
            console.log('Event received:', event);
            updateMetric('eventCount', 1);
        });
        
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        log('intentOutput', `❌ Failed to initialize: ${error.message}`);
    } finally {
        showLoader(false);
    }
}

// Intent Management Functions
window.createIntent = function() {
    try {
        const goal = document.getElementById('intentGoal').value;
        const priority = document.getElementById('intentPriority').value;
        
        const builder = new WasmIntentBuilder(goal);
        currentIntent = builder
            .with_description(`Browser-created intent: ${goal}`)
            .with_priority(priority)
            .build();
        
        log('intentOutput', `✅ Intent created:\nID: ${currentIntent.id}\nGoal: ${currentIntent.goal}\nStatus: ${currentIntent.status}`);
        
        // Record metric
        metricsCollector.record_count("intents_created", 1);
        updateMetric('operationCount', 1);
        
    } catch (error) {
        log('intentOutput', `❌ Error creating intent: ${error.message}`);
    }
};

window.executeIntent = async function() {
    if (!currentIntent) {
        log('intentOutput', '⚠️ No intent created. Create an intent first.');
        return;
    }
    
    try {
        showLoader(true);
        
        // Add steps to the intent
        currentIntent.add_step("fetch_data", "Fetch data from source");
        currentIntent.add_step("process", "Process the data");
        currentIntent.add_step("store", "Store results");
        
        // Simulate execution
        log('intentOutput', '🔄 Executing intent steps...');
        
        await simulateDelay(1000);
        log('intentOutput', '✓ Step 1: Data fetched');
        
        await simulateDelay(1000);
        log('intentOutput', '✓ Step 2: Data processed');
        
        await simulateDelay(1000);
        log('intentOutput', '✓ Step 3: Results stored');
        
        log('intentOutput', `✅ Intent execution completed!`);
        
        // Record execution time
        metricsCollector.record_execution_time("intent_execution", 3000);
        
    } catch (error) {
        log('intentOutput', `❌ Execution failed: ${error.message}`);
    } finally {
        showLoader(false);
    }
};

window.verifyIntent = async function() {
    if (!currentIntent) {
        log('intentOutput', '⚠️ No intent to verify.');
        return;
    }
    
    try {
        const result = await verify_intent(currentIntent.id);
        const verification = JSON.parse(result);
        
        log('intentOutput', 
            `🔍 Verification Result:\n` +
            `Verified: ${verification.verified ? '✅' : '❌'}\n` +
            `Intent ID: ${verification.intent_id}\n` +
            `Method: ${verification.verification_method}\n` +
            `Timestamp: ${new Date(verification.timestamp).toLocaleString()}`
        );
        
    } catch (error) {
        log('intentOutput', `❌ Verification failed: ${error.message}`);
    }
};

// Promise Functions
window.createPromise = function() {
    try {
        const body = document.getElementById('promiseBody').value;
        const promiser = document.getElementById('promiser').value;
        const promisee = document.getElementById('promisee').value;
        
        currentPromise = new WasmPromise(body, promiser, promisee);
        
        log('promiseOutput', 
            `📝 Promise created:\n` +
            `ID: ${currentPromise.id}\n` +
            `From: ${promiser} → To: ${promisee}\n` +
            `Promise: "${body}"\n` +
            `State: ${currentPromise.state}`
        );
        
    } catch (error) {
        log('promiseOutput', `❌ Error creating promise: ${error.message}`);
    }
};

window.acceptPromise = function() {
    if (!currentPromise) {
        log('promiseOutput', '⚠️ No promise to accept.');
        return;
    }
    
    try {
        currentPromise.accept();
        log('promiseOutput', `✅ Promise accepted!\nState: ${currentPromise.state}`);
    } catch (error) {
        log('promiseOutput', `❌ Error accepting promise: ${error.message}`);
    }
};

window.fulfillPromise = function() {
    if (!currentPromise) {
        log('promiseOutput', '⚠️ No promise to fulfill.');
        return;
    }
    
    try {
        currentPromise.fulfill();
        log('promiseOutput', `✅ Promise fulfilled!\nState: ${currentPromise.state}`);
        
        // Update trust for the promiser
        if (trustModel) {
            const promiser = document.getElementById('promiser').value;
            trustModel.update_trust(promiser, true);
        }
        
    } catch (error) {
        log('promiseOutput', `❌ Error fulfilling promise: ${error.message}`);
    }
};

// Trust Model Functions
window.addAgent = function() {
    try {
        const agentId = document.getElementById('agentId').value;
        const trustScore = parseFloat(document.getElementById('trustScore').value);
        
        trustModel.add_agent(agentId, trustScore);
        log('trustOutput', `✅ Agent added:\nID: ${agentId}\nInitial Trust: ${trustScore}`);
        
    } catch (error) {
        log('trustOutput', `❌ Error adding agent: ${error.message}`);
    }
};

window.updateTrust = function(fulfilled) {
    try {
        const agentId = document.getElementById('agentId').value;
        trustModel.update_trust(agentId, fulfilled);
        
        const newTrust = trustModel.get_trust(agentId);
        log('trustOutput', 
            `${fulfilled ? '✅' : '❌'} Trust updated for ${agentId}:\n` +
            `Action: Promise ${fulfilled ? 'fulfilled' : 'violated'}\n` +
            `New Trust Score: ${newTrust.toFixed(3)}`
        );
        
    } catch (error) {
        log('trustOutput', `❌ Error updating trust: ${error.message}`);
    }
};

window.getTrustScore = function() {
    try {
        const agentId = document.getElementById('agentId').value;
        const trust = trustModel.get_trust(agentId);
        const isTrustworthy = trustModel.is_trustworthy(agentId, 0.6);
        
        log('trustOutput', 
            `📊 Trust Report for ${agentId}:\n` +
            `Trust Score: ${trust.toFixed(3)}\n` +
            `Trustworthy (>0.6): ${isTrustworthy ? '✅ Yes' : '❌ No'}`
        );
        
    } catch (error) {
        log('trustOutput', `❌ Error getting trust: ${error.message}`);
    }
};

// Verification Functions
window.verifyCommand = function() {
    try {
        const command = document.getElementById('commandToVerify').value;
        const output = document.getElementById('expectedOutput').value;
        
        const verifier = new WasmCommandVerifier();
        const result = verifier.verify_command(command);
        
        log('verificationOutput', 
            `🔍 Command Verification:\n` +
            `Command: ${command}\n` +
            `Verified: ${result.verified ? '✅' : '❌'}\n` +
            `Safe: ${result.is_safe ? 'Yes' : 'No'}\n` +
            `Confidence: ${(result.confidence * 100).toFixed(0)}%`
        );
        
    } catch (error) {
        log('verificationOutput', `❌ Verification error: ${error.message}`);
    }
};

window.verifyFile = function() {
    try {
        const verifier = new WasmFileSystemVerifier();
        verifier.expect_file("/output/result.json");
        
        const files = ["/output/result.json"];
        const result = verifier.verify_files(files);
        
        log('verificationOutput', 
            `📁 File Verification:\n` +
            `Expected: /output/result.json\n` +
            `Verified: ${result.verified ? '✅' : '❌'}\n` +
            `Files Checked: ${result.files_checked}\n` +
            `Files Found: ${result.files_found}`
        );
        
    } catch (error) {
        log('verificationOutput', `❌ File verification error: ${error.message}`);
    }
};

window.compositeVerify = function() {
    try {
        const composite = new WasmCompositeVerifier();
        
        // Add multiple verification results
        const cmdVerifier = new WasmCommandVerifier();
        const cmdResult = cmdVerifier.verify_command("echo test");
        composite.add_result(cmdResult);
        
        const fileVerifier = new WasmFileSystemVerifier();
        fileVerifier.expect_file("/test.txt");
        const fileResult = fileVerifier.verify_files(["/test.txt"]);
        composite.add_result(fileResult);
        
        const overall = composite.get_overall_result();
        const confidence = composite.confidence_score();
        
        log('verificationOutput', 
            `🔗 Composite Verification:\n` +
            `Strategies Used: 2\n` +
            `Overall Result: ${overall.verified ? '✅ Passed' : '❌ Failed'}\n` +
            `Confidence Score: ${(confidence * 100).toFixed(0)}%\n` +
            `Details: ${JSON.stringify(overall.details, null, 2)}`
        );
        
    } catch (error) {
        log('verificationOutput', `❌ Composite verification error: ${error.message}`);
    }
};

// Observability Functions
window.startObservation = function() {
    try {
        observable.start_operation("demo_operation");
        log('observabilityOutput', '🎬 Started observing operation: demo_operation');
        updateMetric('operationCount', 1);
        
    } catch (error) {
        log('observabilityOutput', `❌ Error starting observation: ${error.message}`);
    }
};

window.recordMetric = function() {
    try {
        const metricName = `metric_${Date.now()}`;
        const value = Math.random() * 100;
        
        metricsCollector.record_count(metricName, value);
        log('observabilityOutput', `📊 Recorded metric:\nName: ${metricName}\nValue: ${value.toFixed(2)}`);
        updateMetric('metricCount', 1);
        
    } catch (error) {
        log('observabilityOutput', `❌ Error recording metric: ${error.message}`);
    }
};

window.emitEvent = function() {
    try {
        const event = new WasmEvent(
            "custom_event",
            JSON.stringify({ timestamp: Date.now(), action: "user_action" }),
            "browser"
        );
        
        eventEmitter.emit(event);
        log('observabilityOutput', `📡 Event emitted:\nType: custom_event\nSource: browser`);
        
    } catch (error) {
        log('observabilityOutput', `❌ Error emitting event: ${error.message}`);
    }
};

window.getSummary = function() {
    try {
        observable.end_operation();
        const summary = observable.get_summary();
        const metrics = metricsCollector.get_summary();
        
        log('observabilityOutput', 
            `📈 Observability Summary:\n` +
            `Observable: ${JSON.stringify(summary, null, 2)}\n` +
            `Metrics: ${JSON.stringify(metrics, null, 2)}`
        );
        
    } catch (error) {
        log('observabilityOutput', `❌ Error getting summary: ${error.message}`);
    }
};

// Helper Functions
function log(elementId, message) {
    const output = document.getElementById(elementId);
    const timestamp = new Date().toLocaleTimeString();
    output.textContent = `[${timestamp}] ${message}\n${output.textContent}`;
}

function showLoader(show) {
    const loader = document.getElementById('loader');
    loader.classList.toggle('active', show);
}

function simulateDelay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

function updateMetric(metricId, increment) {
    const element = document.getElementById(metricId);
    const current = parseInt(element.textContent) || 0;
    element.textContent = current + increment;
}

// Initialize on page load
window.addEventListener('load', initializeWasm);