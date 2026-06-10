#!/usr/bin/env bb
;; protect-checks-hook.bb — Claude Code PreToolUse hook (matcher: Edit|Write).
;; Reads the hook JSON from stdin, extracts the target file path, and exits 2
;; (blocking; stderr is fed back to Claude) if the path is check
;; infrastructure that the agent must never edit unilaterally.
;;
;; Requires babashka (JSON parsing is built in — no jq).
;; Verify the stdin JSON shape against the current hooks docs:
;; https://code.claude.com/docs/en/hooks  (field: .tool_input.file_path)

(require '[cheshire.core :as json]
         '[clojure.string :as str])

;; NOTE: exit codes here follow the Claude Code HOOK contract, not the kit's
;; audit-script dictionary: 0 = allow the tool call, 2 = BLOCK it (stderr is
;; fed back to the agent). Do not "normalize" these.
(def usage "usage: protect-checks-hook.bb   (reads hook JSON on stdin)

Claude Code PreToolUse hook (matcher: Edit|Write). Blocks edits to
protected check infrastructure. Wire via .claude/settings.json — not
normally invoked by hand.

exit codes (Claude Code hook contract): 0 allow | 2 block
example: echo '{\"tool_input\":{\"file_path\":\"mutants.toml\"}}' | protect-checks-hook.bb")

(when (some #{"-h" "--help"} *command-line-args*)
  (println usage) (System/exit 0))

(def protected
  ["proptest-regressions/"     ; append-only known-failure seeds
   ".cargo/mutants.toml"
   "mutants.toml"
   "hex-lint-exceptions.toml"  ; architectural-debt ledger: entries are check-changes
   ".fn-hashes.jsonl"          ; mutation-scoping snapshot: regenerate via fn-hash, never hand-edit
   "tests/golden/"             ; conformance oracles: re-blessing output is a check-change
   "scripts/quality-gate.bb"
   "scripts/trace-audit.bb"
   "scripts/check-guard.bb"
   "scripts/protect-checks-hook.bb"
   ".claude/settings.json"])   ; the hooks themselves

(let [input (try (json/parse-string (slurp *in*) true)
                 (catch Exception _ nil))
      file  (get-in input [:tool_input :file_path])]
  (when (and file (some #(str/includes? file %) protected))
    (binding [*out* *err*]
      (println (str "BLOCKED: '" file "' is protected check infrastructure."))
      (println "Per CLAUDE.md, modifying checks/gates requires explicit human")
      (println "approval as a standalone 'check-change:' commit. State what you")
      (println "want to change and why, then wait for the human."))
    (System/exit 2)))
(System/exit 0)
