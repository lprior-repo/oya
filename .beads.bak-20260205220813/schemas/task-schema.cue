package planner

import "list"

// Task Schema - Validates task JSON before adding to session
// This ensures AI provides correctly structured task data

#Task: {
	// Required fields
	id:          string & =~"^task-[0-9]{3,}$"
	title:       string & =~"^[a-z-]+: .+$" // Format: "component: action"
	type:        "feature" | "bug" | "task" | "epic" | "chore"
	priority:    >=0 & <=4 // 0=critical, 4=backlog
	effort:      "15min" | "30min" | "1hr" | "2hr" | "4hr"
	description: string & !="" // Must not be empty

	// EARS Requirements (Easy Approach to Requirements Syntax)
	ears: {
		// Pattern 1: Ubiquitous - Always true
		ubiquitous: [...string] & list.MinItems(1)
		ubiquitous: [...=~"^THE SYSTEM SHALL .+"]

		// Pattern 2: Event-Driven - Trigger → Response
		event_driven: [...{
			trigger: string & =~"^WHEN .+"
			shall:   string & =~"^THE SYSTEM SHALL .+"
		}] & list.MinItems(1)

		// Pattern 3: State-Driven (optional)
		state_driven?: [...{
			state: string & =~"^WHILE .+"
			shall: string & =~"^THE SYSTEM SHALL .+"
		}]

		// Pattern 4: Optional (optional)
		optional?: [...{
			condition: string & =~"^WHERE .+"
			shall:     string & =~"^THE SYSTEM SHALL .+"
		}]

		// Pattern 5: Unwanted - Must never happen
		unwanted: [...{
			condition:  string & =~"^IF .+"
			shall_not:  string & =~"^THE SYSTEM SHALL NOT .+"
			because:    string & !=""
		}] & list.MinItems(1)

		// Pattern 6: Complex (optional)
		complex?: [...{
			state:   string & =~"^WHILE .+"
			trigger: string & =~"^WHEN .+"
			shall:   string & =~"^THE SYSTEM SHALL .+"
		}]
	}

	// KIRK Contracts (Design by Contract)
	contracts: {
		// Preconditions - What must be true before
		preconditions: [...string] & list.MinItems(1)

		// Postconditions - What must be true after
		postconditions: [...string] & list.MinItems(1)

		// Invariants - Always true
		invariants: [...string] & list.MinItems(1)
	}

	// Tests (ATDD - Acceptance Test-Driven Development)
	tests: {
		// Happy path tests - Required
		happy: [...string] & list.MinItems(2)

		// Error path tests - Required
		error: [...string] & list.MinItems(2)

		// Edge case tests - Recommended
		edge?: [...string]

		// Contract tests - Recommended
		contract?: [...string]
	}

	// Research Requirements (Read before write)
	research?: {
		files?:     [...string]
		patterns?:  [...string]
		questions?: [...string]
	}

	// Inversions (Charlie Munger: "Invert, always invert")
	inversions?: {
		security_failures?: [...{
			failure:    string
			prevention: string
			test_for_it: string
		}]
		usability_failures?: [...{
			failure:    string
			prevention: string
			test_for_it: string
		}]
		data_integrity_failures?: [...{
			failure:    string
			prevention: string
			test_for_it: string
		}]
		integration_failures?: [...{
			failure:    string
			prevention: string
			test_for_it: string
		}]
	}

	// Implementation Tasks
	implementation?: {
		phase_0?: [...string] // Research
		phase_1?: [...string] // Tests first
		phase_2?: [...string] // Implementation
		phase_3?: [...string] // Integration
		phase_4?: [...string] // Verification
	}

	// Context
	context?: {
		related_files?:           [...string]
		similar_implementations?: [...string]
		similar?:                 [...string]  // Alias for similar_implementations
		external_references?:     [...string]
	}

	// Anti-Hallucination Guards
	anti_hallucination?: {
		read_before_write?: [...{
			file:                      string
			must_read_first:           bool
			key_sections_to_understand: [...string]
		}]
		apis_that_exist?: [...string]
		apis_that_do_not_exist?: [...string]
		no_placeholder_values?: [...string]
	}
}

// Quality Thresholds
#QualityScore: {
	contract_score:    >=0 & <=100
	test_score:        >=0 & <=100
	adversarial_score: >=0 & <=100
	overall_score:     >=0 & <=100

	// Minimum thresholds
	contract_score:    >=60  // Design by Contract minimum
	test_score:        >=70  // Test quality minimum
	adversarial_score: >=60  // Red Queen minimum
	overall_score:     >=70  // Overall minimum to pass
}

// Validation Rules
#ValidationResult: {
	valid:   bool
	score?:  #QualityScore
	errors?: [...string]
	warnings?: [...string]
}
