# User API Requirements
# Written in EARS (Easy Approach to Requirements Syntax)

# Ubiquitous Requirements
THE SYSTEM SHALL validate all API inputs against schema
THE SYSTEM SHALL log all API requests with timestamps
THE SYSTEM SHALL return JSON responses with consistent structure

# Event-Driven Requirements (WHEN...SHALL)
WHEN user submits registration form THE SYSTEM SHALL validate email format
WHEN user requests password reset THE SYSTEM SHALL send reset email within 30 seconds
WHEN authentication succeeds THE SYSTEM SHALL return JWT token with 24-hour expiry
WHEN user updates profile THE SYSTEM SHALL record change in audit log

# State-Driven Requirements (WHILE...SHALL)
WHILE user is authenticated THE SYSTEM SHALL allow access to protected endpoints
WHILE rate limit is exceeded THE SYSTEM SHALL return 429 status code
WHILE system is in maintenance mode THE SYSTEM SHALL return 503 status code

# Optional Requirements (WHERE...SHALL)
WHERE user has admin role THE SYSTEM SHALL allow access to admin endpoints
WHERE request includes API key THE SYSTEM SHALL bypass rate limiting
WHERE two-factor is enabled THE SYSTEM SHALL require OTP verification

# Unwanted Behaviors (IF...SHALL NOT)
IF user is banned THEN THE SYSTEM SHALL NOT allow login
IF token is expired THEN THE SYSTEM SHALL NOT authorize requests
IF password has been compromised THEN THE SYSTEM SHALL NOT accept it

# Complex Requirements (WHILE...WHEN...SHALL)
WHILE user is logged in WHEN session expires THE SYSTEM SHALL redirect to login
WHILE in transaction WHEN error occurs THE SYSTEM SHALL rollback all changes
