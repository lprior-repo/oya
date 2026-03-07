# ============================================================================
# BEAD: solitaire-001 - Implement Klondike Solitaire Game
# ============================================================================

id: "solitaire-001"
title: "Rust: Implement Klondike Solitaire game"
type: feature
priority: 1
effort_estimate: "2hr"
labels: [rust, game, feature]

clarification_status: "RESOLVED"

resolved_clarifications:
  - question: "Which solitaire variant?"
    answer: "Klondike (standard solitaire)"
    decided_by: "user request"
    date: "2026-03-06"

assumptions:
  - assumption: "CLI-based interface is acceptable"
    validation_method: "User approved"
    risk_if_wrong: "May need GUI"

ears_requirements:
  ubiquitous:
    - "THE SYSTEM SHALL support standard 52-card deck"
    - "THE SYSTEM SHALL implement Klondike rules"
  
  event_driven:
    - trigger: "WHEN user starts game"
      shall: "THE SYSTEM SHALL deal cards in tableau"
    - trigger: "WHEN user moves card"
      shall: "THE SYSTEM SHALL validate move legality"

# ============================================================================
# SECTION 2: DOMAIN MODEL (The Truth)
# ============================================================================

domain_model:
  entities:
    Card:
      rank: Rank (Ace..King)
      suit: Suit (Hearts|Diamonds|Clubs|Spades)
      face_up: bool
    
    Pile:
      cards: Vec<Card>
      pile_type: PileType
    
    Game:
      tableau: [Pile; 7]
      foundations: [Pile; 4]
      stock: Pile
      waste: Pile

# ============================================================================
# SECTION 3: TEST SPECIFICATIONS
# ============================================================================

test_specifications:
  - name: "deck_has_52_cards"
    given: "new game initialized"
    when: "deck created"
    then: "total cards equals 52"
  
  - name: "valid_move_allowed"
    given: "red 5 on black 6"
    when: "move attempted"
    then: "move succeeds"

# ============================================================================
# SECTION 4: IMPLEMENTATION CONTRACT
# ============================================================================

implementation_contract:
  modules:
    - "src/game.rs - core game logic"
    - "src/card.rs - card types"
    - "src/pile.rs - pile management"
    - "src/main.rs - CLI entry point"
  
  error_handling:
    - "InvalidMove - when move violates rules"
    - "EmptyPile - when drawing from empty pile"
  
  success_criteria:
    - "Can start new game"
    - "Can move cards following rules"
    - "Can win game"
