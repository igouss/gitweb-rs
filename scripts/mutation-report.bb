#!/usr/bin/env bb
;; mutation-report.bb — mutation score from cargo-mutants outcomes, with a
;; per-file survivor breakdown and a committed trend row. The score is the
;; ratchet's truth metric (verification-ratchet Layer 3): of the viable
;; mutants, what fraction did the tests kill.
;;
;;   score = killed / viable          killed = caught + timeout
;;                                    viable = caught + timeout + missed
;; Unviable mutants (the mutated source does not compile) are EXCLUDED — they
;; say nothing about test strength — but counted and surfaced separately.
;; Timeouts count as killed: a mutation that hangs the suite changed behavior
;; enough that the tests noticed.
;;
;; This script SCORES an existing cargo-mutants run; it does not run mutants
;; (that is slow and you scope it per crate / shard). Mirror of the
;; quality-gate split: cargo-mutants runs, this reads the outcomes — exactly
;; as crap-report.bb reads an llvm-cov export.
;;
;; WHERE MUTATION TESTING MAKES SENSE (apply it per capability, not blanket):
;;   - gitweb-domain      (role=domain)            — YES. Pure business logic;
;;                                                   this is mutation testing's home.
;;   - gitweb-render      (role=port-and-adapter)  — yes, real formatting logic.
;;   - gitweb-git/web     (adapters)               — low value: I/O + wiring the
;;                                                   cucumber suites already pin.
;;   - app/config/fixtures/parity (glue + infra)   — SKIP. Mutating composition
;;                                                   roots and test scaffolding is theater.
;; The per-crate table below makes each crate's score visible so this stays a
;; judgement on evidence, not a blanket number. (This is a scoping decision on
;; WHERE to invest in killing survivors — not an `exclude_globs`, which would
;; hide code from the tool and is a check-change per CLAUDE.md.)
;;
;; Requires: babashka, and a cargo-mutants outcomes.json:
;;   export TMPDIR="$HOME/.cache/cargo-mutants"; mkdir -p "$TMPDIR"  # NEVER /tmp
;;   cargo mutants -p <crate> --profile mutants   # writes mutants.out/outcomes.json
;;                                                # (--profile mutants = debuginfo-off build, faster, same score)
;;
;; PROJECT-WIDE score (the 50% target spans all crates, but no single run is
;; tractable): snapshot each crate's outcomes into an archive dir, then
;; aggregate them:
;;   cargo mutants -p gitweb-domain --profile mutants && cp mutants.out/outcomes.json \
;;     target/metrics/mutation/domain.json
;;   ...repeat per crate...
;;   mutation-report.bb --archive target/metrics/mutation --gate 50
;;
;; Usage:
;;   mutation-report.bb [--outcomes FILE] [--archive DIR] [--top N] [--gate N]
;; Outputs:
;;   target/metrics/mutation-report.md   — survivor triage report
;;   metrics/mutation-history.csv        — one trend row per run, committed.
;;                                         Append is IDEMPOTENT per (date, commit).
;; Slow upstream (the mutants run). Not for pre-commit. Run before merge / CI.

(require '[babashka.fs :as fs]
         '[babashka.process :refer [sh]]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def usage "usage: mutation-report.bb [--outcomes FILE] [--archive DIR] [--top N] [--gate N]

Mutation score from cargo-mutants outcomes (killed / viable), with a
per-file survivor breakdown and a committed trend row.

flags:
  --outcomes FILE   a single cargo-mutants outcomes.json
                    (default: mutants.out/outcomes.json)
  --archive DIR     aggregate every *.json in DIR into one project-wide
                    score (per-crate snapshots; overrides --outcomes)
  --top N           survivor rows in the report table (default: 40)
  --gate N          exit 1 if the mutation score is BELOW N percent
  -h, --help        this help

outputs: target/metrics/mutation-report.md, metrics/mutation-history.csv
         (trend row, idempotent per date+commit — commit it), report on stdout.
exit codes: 0 success | 1 gate not met | 2 usage/environment error
example: mutation-report.bb --archive target/metrics/mutation --gate 50")

(defn die-usage! [msg]
  (binding [*out* *err*] (println (str "error: " msg)) (println) (println usage))
  (System/exit 2))

(defn lev [s t]
  (let [s (vec s) t (vec t)]
    (peek (reduce (fn [prev i]
                    (reduce (fn [row j]
                              (conj row (min (inc (peek row))
                                             (inc (nth prev (inc j)))
                                             (+ (nth prev j) (if (= (s i) (t j)) 0 1)))))
                            [(inc i)] (range (count t))))
                  (vec (range (inc (count t)))) (range (count s))))))

(def value-flags #{"--outcomes" "--archive" "--top" "--gate"})
(when (some #{"-h" "--help"} *command-line-args*)
  (println usage) (System/exit 0))
(def args
  (loop [m {} [a & more] *command-line-args*]
    (cond
      (nil? a) m
      (value-flags a) (if (and (first more) (not (str/starts-with? (first more) "-")))
                        (recur (assoc m a (first more)) (rest more))
                        (die-usage! (str a " requires a value")))
      :else (let [sug (first (sort-by #(lev a %) (filter #(<= (lev a %) 2) value-flags)))]
              (die-usage! (str "unknown argument " a
                               (when sug (str " — did you mean " sug "?"))))))))
(def outcomes-path (get args "--outcomes" "mutants.out/outcomes.json"))
(def archive-dir (get args "--archive"))
(def top-n (or (parse-long (get args "--top" "40"))
               (die-usage! "--top expects an integer")))
(def gate (when-let [g (get args "--gate")]
            (or (parse-long g) (die-usage! "--gate expects an integer"))))

;; --- inputs: one outcomes file, or every *.json in an archive dir -----------
(def input-files
  (if archive-dir
    (let [ds (fs/glob archive-dir "*.json")]
      (when-not (seq ds)
        (die-usage! (str "no *.json under " archive-dir " — snapshot per-crate runs first:\n"
                         "  export TMPDIR=\"$HOME/.cache/cargo-mutants\"; mkdir -p \"$TMPDIR\"\n"
                         "  cargo mutants -p <crate> --profile mutants && cp mutants.out/outcomes.json "
                         archive-dir "/<crate>.json")))
      (mapv str ds))
    (do (when-not (fs/exists? outcomes-path)
          (die-usage! (str "no outcomes at " outcomes-path " — produce it with:\n"
                           "  export TMPDIR=\"$HOME/.cache/cargo-mutants\"; mkdir -p \"$TMPDIR\"\n"
                           "  cargo mutants -p <crate> --profile mutants   # NEVER the default /tmp tmpfs\n"
                           "(verification-ratchet skill, Layer 3).")))
        [outcomes-path])))

;; A cargo-mutants outcomes.json carries one Baseline run (summary "Success",
;; scenario without a :Mutant key) plus one entry per mutant. Keep only the
;; mutant scenarios; normalize to {:file :name :summary}.
(defn mutant-outcomes [path]
  (->> (:outcomes (json/parse-string (slurp path) true))
       (keep (fn [o]
               (when-let [m (get-in o [:scenario :Mutant])]
                 {:file (:file m) :name (:name m) :summary (:summary o)})))))

(def all (vec (mapcat mutant-outcomes input-files)))
(when (empty? all)
  (die-usage! "no mutant outcomes found — was this an all-unviable run? Check\nmutants.out/log for 'Disk quota exceeded' (the /tmp tmpfs trap)."))

(def by-summary (frequencies (map :summary all)))
(def caught   (get by-summary "CaughtMutant" 0))
(def missed   (get by-summary "MissedMutant" 0))
(def timeout  (get by-summary "Timeout" 0))
(def unviable (get by-summary "Unviable" 0))
(def killed (+ caught timeout))
(def viable (+ caught timeout missed))
(def score (if (pos? viable) (* 100.0 (/ (double killed) viable)) 0.0))

(defn r1 [x] (format "%.1f" (double x)))

;; --- per-crate score: mutation testing is a per-capability judgement, so the
;; score that matters is per crate (domain earns it; glue/infra do not) -------
(defn crate-of [path]
  (let [segs (str/split (str path) #"/")]
    (if (and (= "crates" (first segs)) (second segs)) (second segs) "<other>")))

(defn score-of [outs]
  (let [f (frequencies (map :summary outs))
        k (+ (get f "CaughtMutant" 0) (get f "Timeout" 0))
        v (+ k (get f "MissedMutant" 0))]
    {:killed k :viable v :missed (get f "MissedMutant" 0)
     :unviable (get f "Unviable" 0)
     :score (if (pos? v) (* 100.0 (/ (double k) v)) 0.0)}))

(def per-crate
  (->> (group-by #(crate-of (:file %)) all)
       (map (fn [[c outs]] (assoc (score-of outs) :crate c)))
       (sort-by :score)))

;; --- survivors, grouped by file (the actionable triage list) ----------------
(def survivors (filter #(= "MissedMutant" (:summary %)) all))
(def survivors-by-file
  (->> survivors
       (group-by :file)
       (sort-by (fn [[_ v]] (- (count v))))))

(def repo-root (str/trim (:out (sh "git" "rev-parse" "--show-toplevel"))))

;; --- markdown report (embeds its own triage playbook) -----------------------
(def report-md
  (str "# Mutation report\n\n"
       "Score: **" (r1 score) "%** killed (" killed "/" viable " viable)\n"
       "caught " caught " · timeout " timeout " · missed " missed
       " · unviable " unviable " (excluded)\n"
       "Sources: " (str/join ", " (map #(str/replace % (str repo-root "/") "") input-files)) "\n\n"
       "For every surviving (missed) mutant, exactly one of:\n"
       "1. Strengthen — add/extend a test that kills it (zero/one/many; assert\n"
       "   the exact value, not just \"does not panic\").\n"
       "2. Report — no requirement constrains this behavior; tell the human and\n"
       "   wait. Do NOT exclude the file, lower the gate, or skip the mutant.\n\n"
       "## Per-crate score (mutation testing is a per-capability judgement)\n\n"
       "| crate | score | killed/viable | missed |\n|---|---:|---:|---:|\n"
       (str/join "\n"
         (for [c per-crate]
           (str "| " (:crate c) " | " (r1 (:score c)) "% | "
                (:killed c) "/" (:viable c) " | " (:missed c) " |")))
       "\n\n"
       "| survivors | file |\n|---:|---|\n"
       (str/join "\n"
         (for [[f v] survivors-by-file]
           (str "| " (count v) " | " f " |")))
       "\n\n## Surviving mutants (top " top-n ")\n\n"
       (str/join "\n"
         (for [s (take top-n survivors)]
           (str "- `" (:file s) "` — " (:name s))))
       "\n"))

(fs/create-dirs (fs/path repo-root "target/metrics"))
(spit (str (fs/path repo-root "target/metrics/mutation-report.md")) report-md)

;; --- trend CSV: idempotent append keyed on (date, commit) -------------------
(def csv-path (str (fs/path repo-root "metrics/mutation-history.csv")))
(def header "date,commit,branch,score_pct,killed,viable,caught,timeout,missed,unviable")
(def today (str (java.time.LocalDate/now)))
(def commit (str/trim (:out (sh "git" "rev-parse" "--short" "HEAD"))))
(def row (str/join "," [today commit
                        (str/trim (:out (sh "git" "rev-parse" "--abbrev-ref" "HEAD")))
                        (r1 score) killed viable caught timeout missed unviable]))
(fs/create-dirs (fs/parent csv-path))
(let [old (if (fs/exists? csv-path) (str/split-lines (slurp csv-path)) [])
      kept (remove #(str/starts-with? % (str today "," commit ",")) old)
      lines (if (seq kept) (concat kept [row]) [header row])]
  (spit csv-path (str (str/join "\n" lines) "\n")))

(println report-md)
(println (str "Report:  target/metrics/mutation-report.md\nHistory: metrics/mutation-history.csv (commit it)"))

;; --- SCREAM the verdict to the console (loud — the last thing on screen).
;; Color only when stdout is an interactive console and NO_COLOR is unset, so a
;; piped/CI capture stays clean text (verification-ratchet: a gate must be
;; observable). ------------------------------------------------------------
(def color? (and (nil? (System/getenv "NO_COLOR")) (some? (System/console))))
(defn paint [codes s] (if color? (str "[" codes "m" s "[0m") s))
(def graded? (some? gate))
(def pass? (or (not graded?) (>= score gate)))
(def hue (cond (not graded?) "36" pass? "32" :else "31"))      ; cyan / green / red
(def bar (paint (str "1;" hue) (apply str (repeat 60 "█"))))
(defn mark [ok] (if ok (paint "1;32" "✓") (paint "1;31" "✗")))
(def verdict (cond (not graded?) "" pass? "  PASS ✓" :else "  FAIL ✗"))
(println)
(println bar)
(println (paint (str "1;" hue)
                (format "  MUTATION SCORE: %.1f%%%s%s" score verdict
                        (if graded? (str "   (gate " gate "%)") ""))))
(println (format "  killed %d/%d viable  ·  missed %d  ·  unviable %d excluded"
                 killed viable missed unviable))
(println bar)
(doseq [c per-crate]
  (println (format "  %-12s %5.1f%%  %s   %d/%d  (%d missed)"
                   (:crate c) (:score c)
                   (mark (or (not graded?) (>= (:score c) gate)))
                   (:killed c) (:viable c) (:missed c))))
(println)

;; --- optional gate -----------------------------------------------------------
(when (and gate (< score gate))
  (binding [*out* *err*]
    (println (paint "1;31" (str "MUTATION GATE FAILED — score " (r1 score)
                                "% is below " gate "%.")))
    (println (str missed " surviving mutant(s) to triage. Strengthen a test that"))
    (println "kills each (with the case it pins) or report it — never weaken the gate."))
  (System/exit 1))
