/**
 * Intent CLI JSON Integration Examples
 *
 * Practical examples showing how to integrate Intent CLI JSON output
 * into TypeScript/Node.js applications, CI/CD pipelines, and automation.
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import type {
  JsonResponse,
  QualityResponse,
  CoverageResponse,
  CheckResponse,
  GapsResponse,
  parseIntentOutput,
  isSuccess,
  getNextActions,
} from '../schema/intent-cli';

const execAsync = promisify(exec);

// ============================================================================
// Basic Command Execution
// ============================================================================

/**
 * Execute an Intent CLI command and parse JSON output
 */
async function runIntentCommand<T>(
  command: string,
  args: string[]
): Promise<JsonResponse<T>> {
  const fullCommand = `intent ${command} ${args.join(' ')} --json=true`;
  const { stdout } = await execAsync(fullCommand);
  return JSON.parse(stdout) as JsonResponse<T>;
}

// ============================================================================
// Quality Gate Example
// ============================================================================

/**
 * Quality gate that fails if spec score is below threshold
 */
async function qualityGate(
  specPath: string,
  threshold: number = 80
): Promise<void> {
  console.log(`Running quality gate (threshold: ${threshold})...`);

  const response = await runIntentCommand<QualityResponse['data']>(
    'quality',
    [specPath]
  );

  if (!response.success) {
    const errors = response.errors.map(e => e.message).join('\n');
    throw new Error(`Quality check failed:\n${errors}`);
  }

  const score = response.data.overall_score;
  console.log(`Quality score: ${score}/100`);

  if (score < threshold) {
    console.error(`\n❌ Quality gate failed: ${score} < ${threshold}`);
    console.error('\nIssues:');
    response.data.issues.forEach(issue => console.error(`  • ${issue}`));

    console.error('\nSuggestions:');
    response.data.suggestions.forEach(s => console.error(`  • ${s}`));

    process.exit(1);
  }

  console.log(`✅ Quality gate passed: ${score} >= ${threshold}`);

  // Follow next actions
  if (response.next_actions.length > 0) {
    console.log('\nSuggested next steps:');
    response.next_actions.forEach(action => {
      console.log(`  ${action.command}`);
      console.log(`    → ${action.reason}`);
    });
  }
}

// ============================================================================
// Automated Workflow Example
// ============================================================================

/**
 * Complete KIRK analysis workflow with automated reporting
 */
async function kirkAnalysis(specPath: string): Promise<void> {
  console.log('Starting KIRK analysis...\n');

  // Run all KIRK commands in parallel
  const [quality, coverage, gaps, invert, effects] = await Promise.all([
    runIntentCommand('quality', [specPath]),
    runIntentCommand('coverage', [specPath]),
    runIntentCommand('gaps', [specPath]),
    runIntentCommand('invert', [specPath]),
    runIntentCommand('effects', [specPath]),
  ]);

  // Generate report
  console.log('=== KIRK Analysis Report ===\n');

  // Quality scores
  console.log('Quality Scores:');
  console.log(`  Overall:       ${quality.data.overall_score}/100`);
  console.log(`  Coverage:      ${quality.data.coverage_score}/100`);
  console.log(`  Clarity:       ${quality.data.clarity_score}/100`);
  console.log(`  Testability:   ${quality.data.testability_score}/100`);
  console.log(`  AI Readiness:  ${quality.data.ai_readiness_score}/100`);

  // Coverage analysis
  console.log('\nCoverage:');
  console.log(`  Overall:       ${coverage.data.overall_score.toFixed(1)}%`);
  console.log(`  OWASP:         ${coverage.data.owasp.score.toFixed(1)}%`);
  console.log(`  Edge Cases:    ${coverage.data.edge_cases.tested.length} tested`);
  console.log(`                 ${coverage.data.edge_cases.suggested.length} suggested`);

  // Gaps summary
  console.log('\nGaps Detected:');
  console.log(`  Total:         ${gaps.data.total_gaps}`);
  console.log(`  Critical:      ${gaps.data.severity_breakdown.critical}`);
  console.log(`  High:          ${gaps.data.severity_breakdown.high}`);
  console.log(`  Medium:        ${gaps.data.severity_breakdown.medium}`);
  console.log(`  Low:           ${gaps.data.severity_breakdown.low}`);

  // Inversion analysis
  console.log('\nFailure Mode Analysis:');
  console.log(`  Score:         ${invert.data.score.toFixed(1)}/100`);
  console.log(`  Security:      ${invert.data.security_gaps.length} gaps`);
  console.log(`  Usability:     ${invert.data.usability_gaps.length} gaps`);
  console.log(`  Integration:   ${invert.data.integration_gaps.length} gaps`);

  // Effects analysis
  console.log('\nSecond-Order Effects:');
  console.log(`  Total:         ${effects.data.total_second_order_effects}`);
  console.log(`  Coverage:      ${effects.data.coverage_score.toFixed(1)}%`);
  console.log(`  Orphans:       ${effects.data.orphaned_resources.length}`);

  // Critical issues
  const criticalGaps = gaps.data.security_gaps.filter(
    g => g.severity === 'critical'
  );
  if (criticalGaps.length > 0) {
    console.log('\n⚠️  Critical Security Gaps:');
    criticalGaps.forEach(gap => {
      console.log(`  • ${gap.description}`);
      console.log(`    Fix: ${gap.suggestion}`);
    });
  }

  console.log('\n=== End Report ===');
}

// ============================================================================
// Test Execution with Feedback Loop
// ============================================================================

/**
 * Run tests and generate fix beads for failures
 */
async function testWithFeedback(specPath: string): Promise<void> {
  console.log('Running tests...\n');

  const checkResponse = await runIntentCommand<CheckResponse['data']>(
    'check',
    [specPath]
  );

  console.log('Test Results:');
  console.log(`  Total:    ${checkResponse.data.total}`);
  console.log(`  Passed:   ${checkResponse.data.passed}`);
  console.log(`  Failed:   ${checkResponse.data.failed}`);
  console.log(`  Skipped:  ${checkResponse.data.skipped}`);
  console.log(`  Duration: ${checkResponse.data.duration_ms}ms`);

  if (checkResponse.data.failed > 0) {
    console.log('\n❌ Tests failed. Generating fix beads...\n');

    // Save check results to file
    const fs = require('fs');
    const resultsFile = 'check-results.json';
    fs.writeFileSync(resultsFile, JSON.stringify(checkResponse, null, 2));

    // Generate feedback
    const feedbackResponse = await runIntentCommand('feedback', [
      '--results',
      resultsFile,
    ]);

    console.log('Fix Beads Generated:');
    feedbackResponse.data.fix_beads.forEach((bead, i) => {
      console.log(`\n${i + 1}. ${bead.behavior_name} (Priority: ${bead.priority})`);
      console.log(`   Feature: ${bead.feature}`);
      console.log(`   Type: ${bead.failure_type}`);
      console.log(`   Issue: ${bead.description}`);
      console.log(`   Fix: ${bead.fix_suggestion}`);
    });

    process.exit(1);
  }

  console.log('\n✅ All tests passed!');
}

// ============================================================================
// Workflow Automation with Next Actions
// ============================================================================

/**
 * Execute a workflow by following next_actions suggestions
 */
async function autoWorkflow(
  specPath: string,
  maxDepth: number = 3
): Promise<void> {
  console.log('Starting automated workflow...\n');

  let response = await runIntentCommand('quality', [specPath]);
  let depth = 0;

  while (response.next_actions.length > 0 && depth < maxDepth) {
    console.log(`\n=== Step ${depth + 1} ===`);
    console.log(`Command: ${response.command}`);
    console.log(`Action: ${response.action}`);

    const nextAction = response.next_actions[0];
    console.log(`\nNext: ${nextAction.command}`);
    console.log(`Reason: ${nextAction.reason}`);

    // Parse command (simplified - real implementation would be more robust)
    const parts = nextAction.command.split(' ');
    const command = parts[1]; // Skip 'intent'
    const args = parts.slice(2).filter(arg => !arg.startsWith('--'));

    response = await runIntentCommand(command, args);
    depth++;
  }

  console.log('\n=== Workflow Complete ===');
}

// ============================================================================
// CI/CD Integration Example
// ============================================================================

/**
 * Complete CI/CD check suitable for GitHub Actions, GitLab CI, etc.
 */
async function cicdCheck(
  specPath: string,
  config: {
    qualityThreshold?: number;
    coverageThreshold?: number;
    allowCriticalGaps?: boolean;
  } = {}
): Promise<void> {
  const {
    qualityThreshold = 80,
    coverageThreshold = 70,
    allowCriticalGaps = false,
  } = config;

  console.log('=== CI/CD Quality Check ===\n');

  let failed = false;

  // Quality check
  const quality = await runIntentCommand('quality', [specPath]);
  const qualityScore = quality.data.overall_score;
  console.log(`Quality: ${qualityScore}/100 (threshold: ${qualityThreshold})`);

  if (qualityScore < qualityThreshold) {
    console.error(`  ❌ Below threshold`);
    failed = true;
  } else {
    console.log(`  ✅ Passed`);
  }

  // Coverage check
  const coverage = await runIntentCommand('coverage', [specPath]);
  const coverageScore = coverage.data.overall_score;
  console.log(
    `\nCoverage: ${coverageScore.toFixed(1)}% (threshold: ${coverageThreshold})`
  );

  if (coverageScore < coverageThreshold) {
    console.error(`  ❌ Below threshold`);
    failed = true;
  } else {
    console.log(`  ✅ Passed`);
  }

  // Critical gaps check
  const gaps = await runIntentCommand('gaps', [specPath]);
  const criticalCount = gaps.data.severity_breakdown.critical;
  console.log(`\nCritical Gaps: ${criticalCount}`);

  if (criticalCount > 0 && !allowCriticalGaps) {
    console.error(`  ❌ Critical gaps not allowed`);
    gaps.data.security_gaps
      .filter(g => g.severity === 'critical')
      .forEach(gap => {
        console.error(`    • ${gap.description}`);
      });
    failed = true;
  } else {
    console.log(`  ✅ Passed`);
  }

  // Tests
  const check = await runIntentCommand('check', [specPath]);
  console.log(
    `\nTests: ${check.data.passed}/${check.data.total} passed`
  );

  if (check.data.failed > 0) {
    console.error(`  ❌ ${check.data.failed} tests failed`);
    failed = true;
  } else {
    console.log(`  ✅ All tests passed`);
  }

  if (failed) {
    console.error('\n❌ CI/CD check failed');
    process.exit(1);
  }

  console.log('\n✅ CI/CD check passed');
}

// ============================================================================
// Error Handling Example
// ============================================================================

/**
 * Robust error handling for Intent CLI commands
 */
async function robustExecution(specPath: string): Promise<void> {
  try {
    const response = await runIntentCommand('quality', [specPath]);

    if (!response.success) {
      console.error('Command failed:');
      response.errors.forEach(error => {
        console.error(`  [${error.code}] ${error.message}`);
        if (error.location) {
          console.error(`    at ${error.location}`);
        }
        if (error.fix_hint) {
          console.error(`    hint: ${error.fix_hint}`);
        }
        if (error.fix_command) {
          console.error(`    fix: ${error.fix_command}`);
        }
      });

      // Attempt auto-fix if suggested
      const fixableErrors = response.errors.filter(e => e.fix_command);
      if (fixableErrors.length > 0) {
        console.log('\nAttempting auto-fix...');
        for (const error of fixableErrors) {
          console.log(`Running: ${error.fix_command}`);
          // Execute fix command (simplified)
        }
      }

      process.exit(response.metadata.exit_code);
    }

    console.log('✅ Success');
    console.log(`Correlation ID: ${response.metadata.correlation_id}`);
  } catch (error) {
    console.error('Execution error:', error);
    process.exit(4);
  }
}

// ============================================================================
// Main Examples
// ============================================================================

async function main() {
  const specPath = process.argv[2] || 'examples/user-api.cue';

  // Uncomment to run different examples:

  // await qualityGate(specPath, 80);
  // await kirkAnalysis(specPath);
  // await testWithFeedback(specPath);
  // await autoWorkflow(specPath);
  // await cicdCheck(specPath);
  await robustExecution(specPath);
}

// Run if executed directly
if (require.main === module) {
  main().catch(console.error);
}

// Export for use as library
export {
  runIntentCommand,
  qualityGate,
  kirkAnalysis,
  testWithFeedback,
  autoWorkflow,
  cicdCheck,
  robustExecution,
};
