# Signal Handler Refactoring - Test Failure Root Cause & Solution

## Problem Summary

3 test failures in factory_supervisor_test.gleam caused by global state mutation in signal handling. Every test manipulates the VM's global `erl_signal_server`, creating race conditions that crash the test runner.

## Failing Tests

1. `supervisor_starts_test` - Test crashes during signal handler setup/teardown
2. Module-level crash - factory_supervisor_test module fails to complete
3. Test runner crash - Exit(Killed) from signal handler interference

## Root Cause Analysis

**Global State Mutation in Tests:**

Every test that starts a supervisor manipulates the VM's global `erl_signal_server`:

- `factory_supervisor.gleam:164` - `signal_handler.setup()` installs custom handler
- `factory_supervisor.gleam:353` - `signal_handler.teardown()` removes handler

With 496 passing tests, hundreds of install/remove cycles race for the same global resource, corrupting the test runner's own signal handling and causing crashes.

**The Race Condition:**

```
Test 1: install handler -> test runs -> remove handler -> restore default
Test 2:                 install handler -> test runs -> remove handler -> restore default
Test N: install handler -> CRASH (default handler already removed by another test)
```

## BEAM Concurrency Best Practices

### Separation of Concerns (per Erlang OTP Design Principles)

**Application Controller:**
- Manages application lifecycle
- Coordinates startup/shutdown
- Handles OS signals
- Created by application:start/2

**Supervisors:**
- Purely manage process trees
- Start, monitor, restart children
- No global resource management
- Should be testable and instantiable multiple times

**Signal Handling:**
- Application-level concern
- Installed ONCE at VM/application boot
- Not supervisor responsibility

### References

From [Erlang OTP Design Principles](https://www.erlang.org/doc/system/design_principles.html):

> "The application controller creates an application master for the application, which establishes itself as the group leader of all processes in the application."

From [Supervisor Behaviour](https://www.erlang.org/doc/system/sup_princ.html):

> "A supervisor is responsible for starting, stopping and monitoring its child processes. The basic idea is to keep child processes alive by restarting them when necessary."

From [Graceful Shutdown Best Practices](https://ellispritchard.medium.com/graceful-shutdown-on-kubernetes-with-signals-erlang-otp-20-a22325e8ae98):

> "The erl_signal_server is a gen_event manager that receives messages in the form of atoms such as sigterm when signals are sent to the BEAM OS process. Signal handlers should be installed once at application startup, not per-supervisor instance."

## Architectural Violation

`factory_supervisor` violates single responsibility principle by being both:

1. **Reusable OTP supervisor** - should be testable, instantiable multiple times
2. **Application entry point** - manages global signals

This conflation breaks:
- Testability (tests fight over global state)
- Process isolation (supervisor instances aren't independent)
- OTP design principles (supervisors shouldn't manage application-level concerns)

## Bulletproof Solution

### Architecture Layers

**Layer 1: Pure Supervisor (factory_supervisor.gleam)**
- Pure OTP supervisor - no signal handling
- Testable, instantiable multiple times
- Manages process tree only

**Layer 2: Application Controller (factory_application.gleam)**
- Installs signal handlers ONCE at application boot
- Starts root supervisor
- Waits for shutdown signals
- Triggers graceful shutdown on supervisor

### Implementation Changes

#### 1. factory_supervisor.gleam

**Remove from Started record:**
```gleam
// DELETE THIS FIELD:
signal_handler_subject: Subject(signal_handler.SignalHandlerMessage),
```

**Remove from start_link():**
```gleam
// DELETE LINES 163-164:
let signal_handler_subject = process.new_subject()
let _ = signal_handler.setup(signal_handler_subject)
```

**Remove from graceful_shutdown():**
```gleam
// DELETE LINE 353:
signal_handler.teardown()
```

**Delete unused function:**
```gleam
// DELETE ENTIRE FUNCTION (lines 213-217):
pub fn start_and_wait(config: SupervisorConfig) -> Result(Nil, InitFailed) {
  use started <- result.try(start_link(config))
  wait_for_shutdown(started)
  Ok(Nil)
}

// DELETE wait_for_shutdown and wait_for_shutdown_loop functions
```

**Keep these functions (they're correct):**
- `start_link()` - pure supervisor startup
- `shutdown()` / `graceful_shutdown()` - synchronous cleanup
- All accessor functions

#### 2. Create factory_application.gleam

```gleam
//// Application controller - manages signal handling and supervisor lifecycle
////
//// Follows OTP application behavior pattern: install signals once,
//// start supervisor tree, wait for shutdown signal, cleanup gracefully.

import factory_supervisor
import gleam/dict
import gleam/erlang/process.{type Subject}
import logging
import signal_handler

const shutdown_timeout_ms = 1000

/// Start application: install signal handlers, start supervisor, wait for shutdown
pub fn start_and_wait(
  config: factory_supervisor.SupervisorConfig,
) -> Result(Nil, factory_supervisor.InitFailed) {
  // Install signal handlers ONCE at application level
  let signal_handler_subject = process.new_subject()
  case signal_handler.setup(signal_handler_subject) {
    Ok(Nil) -> Nil
    Error(Nil) -> {
      logging.log(
        logging.Error,
        "Failed to install signal handlers, continuing anyway",
        dict.new(),
      )
      Nil
    }
  }

  // Start supervisor tree
  use started <- result.try(factory_supervisor.start_link(config))

  // Wait for shutdown signal
  wait_for_shutdown(started, signal_handler_subject)

  // Cleanup
  signal_handler.teardown()

  Ok(Nil)
}

fn wait_for_shutdown(
  started: factory_supervisor.Started,
  signal_subject: Subject(signal_handler.SignalHandlerMessage),
) -> Nil {
  logging.log(logging.Info, "Waiting for shutdown signal", dict.new())
  wait_for_shutdown_loop(started, signal_subject, 0)
}

fn wait_for_shutdown_loop(
  started: factory_supervisor.Started,
  signal_subject: Subject(signal_handler.SignalHandlerMessage),
  iteration: Int,
) -> Nil {
  case process.receive(signal_subject, shutdown_timeout_ms) {
    Ok(signal_handler.SignalReceived(signal)) -> {
      let signal_name = case signal {
        signal_handler.Sigterm -> "SIGTERM"
        signal_handler.Sigint -> "SIGINT"
      }
      logging.log(
        logging.Info,
        "Received " <> signal_name <> ", initiating shutdown",
        dict.new(),
      )
      signal_bus.broadcast(
        factory_supervisor.get_signal_bus(started),
        signal_bus.ShutdownRequested,
      )
      factory_supervisor.shutdown(started)
    }
    Error(Nil) -> {
      case iteration % 10 {
        0 ->
          logging.log(
            logging.Debug,
            "Still waiting for shutdown signal",
            dict.new(),
          )
        _ -> Nil
      }
      wait_for_shutdown_loop(started, signal_subject, iteration + 1)
    }
  }
}
```

#### 3. Update factory.gleam (if needed)

If there's a daemon/server mode that uses `start_and_wait`, update it to use `factory_application.start_and_wait()` instead of `factory_supervisor.start_and_wait()`.

### Benefits

**Correctness:**
- Tests never touch signal handlers (no global state mutation)
- Zero race conditions (handlers installed once globally)
- Process isolation maintained per BEAM principles

**Maintainability:**
- Clear separation of concerns
- Supervisors are pure, reusable components
- Application lifecycle in dedicated module

**Testability:**
- Supervisor tests don't interfere with each other
- No test runner crashes from signal handler conflicts
- Can instantiate multiple supervisors in same test

**Compliance:**
- Follows OTP design principles
- Matches Erlang application behavior pattern
- Aligns with BEAM ecosystem best practices

## Migration Path

1. Create `factory_application.gleam` with signal handling logic
2. Update `factory_supervisor.gleam` to remove signal handling
3. Update any callers of `factory_supervisor.start_and_wait()` to use `factory_application.start_and_wait()`
4. Run tests - all 3 failures should be fixed
5. Verify no regressions in production daemon mode

## Expected Outcome

After refactoring:
- All 499 tests pass (496 + 3 previously failing)
- No test runner crashes
- Supervisor can be instantiated multiple times safely
- Application-level signal handling works correctly in production

## References

- [Erlang OTP Design Principles](https://www.erlang.org/doc/system/design_principles.html)
- [Supervisor Behaviour](https://www.erlang.org/doc/system/sup_princ.html)
- [Application Module](https://www.erlang.org/doc/apps/kernel/application.html)
- [Graceful Shutdown on Kubernetes with Erlang OTP](https://ellispritchard.medium.com/graceful-shutdown-on-kubernetes-with-signals-erlang-otp-20-a22325e8ae98)
- [Learn You Some Erlang - Supervisors](https://learnyousomeerlang.com/supervisors)
- [Learn You Some Erlang - Building OTP Applications](https://learnyousomeerlang.com/building-otp-applications)
