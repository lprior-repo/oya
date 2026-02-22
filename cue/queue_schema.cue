package runtimevalidation

#Rfc3339: string & =~"^[0-9]{4}-[0-9]{2}-[0-9]{2}T"

#QueueArtifact: {
  workspace:      string & !=""
  workspace_path: string & !=""
  queue_command:  string & !=""
  queue_passed:   bool
  queue_exit_code: int
  queue_output:   string
  add_command:    string & !=""
  add_passed:     bool
  add_exit_code:  int
  add_output:     string
  recorded_at:    #Rfc3339
}

#LockArtifact: {
  run_id:      string & !=""
  bead_id:     string & =~"^src-[a-z0-9]+$"
  workspace:   string & !=""
  lock_state:  "acquired" | "released" | "contended"
  holder:      string & !=""
  acquired_at: #Rfc3339
  expires_at?: #Rfc3339
}

#ConflictArtifact: {
  run_id:      string & !=""
  bead_id:     string & =~"^src-[a-z0-9]+$"
  workspace:   string & !=""
  conflict_id: string & !=""
  reason_code: string & !=""
  route:       "implementation" | "ship_gate" | "terminal"
  detected_at: #Rfc3339
  details?:    string
}
