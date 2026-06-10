#!/usr/bin/env bb
;; complexity-gate.bb — fast pre-commit complexity check on STAGED .rs files.
;; The fast half of the two-speed CRAP split (verification-ratchet Layer 4):
;; complexity-only here, the slow coverage×complexity join is crap-report.bb.
;;
;; Thresholds are deliberately loose — this gate catches egregious cases,
;; it does not nag every refactor. Override per-commit (paper trail in the
;; commit body, not --no-verify):
;;   CYC_MAX=999 COG_MAX=999 git commit ...
;;
;; Missing-tool policy (quality-gates skill): if rust-code-analysis-cli is
;; not installed, skip silently — a contributor without the optional linter
;; must still be able to commit. CI and quality-gate.bb are where missing
;; tools become errors.

(require '[babashka.fs :as fs]
         '[babashka.process :refer [sh]]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def usage "usage: complexity-gate.bb [file.rs ...]

Fast pre-commit complexity check. With no args, checks STAGED .rs files;
with args, checks the named files (CI / testing).

env:
  CYC_MAX   cyclomatic threshold (default 15)
  COG_MAX   cognitive threshold (default 25)
  Override per-commit (state why in the commit body, not --no-verify):
  CYC_MAX=999 COG_MAX=999 git commit ...

exit codes: 0 clean (also: tool missing, nothing staged) | 1 violations |
            2 usage error
example: CYC_MAX=10 complexity-gate.bb src/lib.rs")

(when (some #{"-h" "--help"} *command-line-args*)
  (println usage) (System/exit 0))
(doseq [a *command-line-args* :when (str/starts-with? a "-")]
  (binding [*out* *err*]
    (println (str "error: unknown flag " a " (files are positional)"))
    (println) (println usage))
  (System/exit 2))

(def cyc-max (parse-long (or (System/getenv "CYC_MAX") "15")))
(def cog-max (parse-long (or (System/getenv "COG_MAX") "25")))

;; Missing-tool policy (quality-gates skill): skip silently — a contributor
;; without the optional linter must still be able to commit.
(when-not (fs/which "rust-code-analysis-cli")
  (System/exit 0))

(def staged
  (if (seq *command-line-args*)
    (vec *command-line-args*)                       ; explicit files (CI / testing)
    (->> (str/split-lines (:out (sh "git" "diff" "--cached" "--name-only"
                                    "--diff-filter=ACMR" "--" "*.rs")))
         (remove str/blank?))))

(when (empty? staged) (System/exit 0))

(defn walk-spaces [node]
  (cons node (mapcat walk-spaces (:spaces node))))

(def violations
  (vec (for [f staged
             :when (fs/exists? f)
             :let [{:keys [exit out]} (sh "rust-code-analysis-cli" "-p" f "-m" "-O" "json")]
             :when (and (zero? exit) (not (str/blank? out)))
             space (walk-spaces (json/parse-string out true))
             :when (contains? #{"function" "closure"} (:kind space))
             :let [cyc (int (or (get-in space [:metrics :cyclomatic :sum]) 0))
                   cog (int (or (get-in space [:metrics :cognitive :sum]) 0))]
             :when (or (> cyc cyc-max) (> cog cog-max))]
         (str "  " f ":" (:start_line space) "  " (or (:name space) "<anon>")
              "  cyc=" cyc "  cog=" cog))))

(when (seq violations)
  (binding [*out* *err*]
    (println "--- complexity gate FAILED ---")
    (println (str "Functions exceed cyclomatic>" cyc-max " or cognitive>" cog-max ":"))
    (println)
    (doseq [v violations] (println v))
    (println)
    (println "Fix: Extract Function on the branchy parts (tdd-cycle REFACTOR).")
    (println (str "Override (state why in the commit body): "
                  "CYC_MAX=999 COG_MAX=999 git commit ..."))
    (println "Full coverage×complexity report: crap-report.bb"))
  (System/exit 1))
