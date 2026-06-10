#!/usr/bin/env bb
;; quality-gate.bb — coverage + CRAP gate. Run after tests are green and the
;; mutation pass is triaged. Requires: babashka, cargo-llvm-cov, cargo-crap.
;;   cargo install cargo-llvm-cov cargo-crap
;;
;; NOTE: cargo-crap is a young tool — verify flag names against
;; `cargo crap --help` on first use and pin the version in CI.

(require '[babashka.process :refer [shell]])

(def crap-threshold (or (System/getenv "CRAP_THRESHOLD") "30"))
(def lcov "target/lcov.info")

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

(println "== 1/3 coverage (cargo-llvm-cov → lcov) ==")
(run! "cargo" "llvm-cov" "--lcov" "--output-path" lcov)

(println (str "== 2/3 CRAP gate (threshold: " crap-threshold ") =="))
;; cargo-crap reads the lcov report and computes, per function:
;;   CRAP(m) = CC(m)^2 * (1 - cov(m))^3 + CC(m)
;; Exit non-zero if any function exceeds the threshold.
(run! "cargo" "crap")

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
