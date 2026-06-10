#!/usr/bin/env bb
;; quality-gate.bb — coverage + CRAP gate. Run after tests are green and the
;; mutation pass is triaged.
;; Requires: babashka, cargo-llvm-cov, rust-code-analysis-cli, hex-lint.
;;   cargo install cargo-llvm-cov rust-code-analysis-cli
;; CRAP is computed in-kit by crap-report.bb (llvm-cov × rust-code-analysis
;; joined per function by file + line-range overlap) — no third-party
;; scorer to trust.

(require '[babashka.fs :as fs]
         '[babashka.process :refer [shell]])

(def crap-threshold (or (System/getenv "CRAP_THRESHOLD") "30"))
(def cov-json "target/metrics/coverage.json")
(fs/create-dirs "target/metrics")

(defn run! [& args]
  (let [{:keys [exit]} (apply shell {:continue true} args)]
    (when-not (zero? exit)
      (binding [*out* *err*]
        (println (str "quality-gate: '" (clojure.string/join " " args)
                      "' failed with exit " exit)))
      (System/exit exit))))

(println "== 0/3 architecture (hex-lint role matrix) ==")
;; Cheapest gate runs first: reads Cargo metadata only, no build, no tests.
;; Fails on any forbidden cross-role dependency edge and on stale entries in
;; hex-lint-exceptions.toml (that file is protected check infrastructure —
;; adding an exception is a check-change, human-approved).
(run! "hex-lint")

(println "== 1/3 coverage (cargo-llvm-cov → JSON export) ==")
(run! "cargo" "llvm-cov" "--json" "--output-path" cov-json)

(println (str "== 2/3 CRAP gate (threshold: " crap-threshold ") =="))
;; crap-report.bb joins the coverage export with rust-code-analysis
;; complexity per function (file + line-range overlap, never names) and
;; computes CRAP(m) = CC(m)^2 * (1 - cov(m))^3 + CC(m).
;; Exits non-zero if any function exceeds the threshold; also writes the
;; top-N report and appends the trend row to metrics/history.csv.
(run! "bb" (str (fs/parent *file*) "/crap-report.bb")
      "--coverage" cov-json "--gate" crap-threshold)

(println "
Gate semantics (CLAUDE.md / verification-ratchet Layer 4):
  - Function over threshold  → refactor (Extract Function, on green,
    tests untouched) OR report to the human as essential complexity.
  - NEVER: raise the threshold, or pad coverage with weak tests to
    shrink the (1-cov)^3 term. Coverage padding is check-gaming and
    the mutation pass will expose it anyway.

Reminder: this gate does NOT replace `cargo mutants`. CRAP measures
complexity-vs-coverage (the refactor signal); mutation score measures
whether your tests would notice wrong code (the test-strength signal).")
