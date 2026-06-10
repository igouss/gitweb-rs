#!/usr/bin/env bb
;; crap-report.bb — per-function CRAP from llvm-cov coverage × rust-code-analysis
;; complexity. CRAP(f) = cyc² × (1−cov)³ + cyc.
;;
;; The join is by FILE + LINE-RANGE OVERLAP, never by function name: Rust
;; name mangling means names do not line up between tools (field-proven rule,
;; verification-ratchet Layer 4).
;;
;; Requires: babashka, rust-code-analysis-cli, and an llvm-cov JSON export:
;;   cargo install rust-code-analysis-cli cargo-llvm-cov
;;   cargo llvm-cov nextest --workspace --json --output-path target/metrics/coverage.json
;;
;; Usage:
;;   crap-report.bb --coverage target/metrics/coverage.json [--top 20]
;;                  [--gate 30]   # exit 1 if any function exceeds the threshold
;; Outputs:
;;   target/metrics/crap-report.md   — top-N report with triage playbook
;;   metrics/history.csv             — one trend row per run, committed.
;;                                     Append is IDEMPOTENT per (date, commit).
;; Slow (full coverage run upstream). Not for pre-commit — that's
;; complexity-gate.bb. Run via `just metrics` / quality-gate.bb / CI.

(require '[babashka.fs :as fs]
         '[babashka.process :refer [shell sh]]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def usage "usage: crap-report.bb [--coverage FILE] [--top N] [--gate N]

Per-function CRAP from llvm-cov coverage × rust-code-analysis complexity,
joined by file + line-range overlap (never by function name).

flags:
  --coverage FILE   llvm-cov JSON export
                    (default: target/metrics/coverage.json; produce it with
                    cargo llvm-cov nextest --workspace --json --output-path <FILE>)
  --top N           rows in the report table (default: 20)
  --gate N          exit 1 if any function's CRAP exceeds N (e.g. 30)
  -h, --help        this help

outputs: target/metrics/crap-report.md, metrics/history.csv (trend row,
idempotent per date+commit — commit it), report on stdout.
exit codes: 0 success | 1 gate exceeded | 2 usage/environment error
example: crap-report.bb --coverage target/metrics/coverage.json --gate 30")

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

(def value-flags #{"--coverage" "--top" "--gate"})
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
(def cov-path (get args "--coverage" "target/metrics/coverage.json"))
(def top-n (or (parse-long (get args "--top" "20"))
               (die-usage! "--top expects an integer")))
(def gate (when-let [g (get args "--gate")]
            (or (parse-long g) (die-usage! "--gate expects an integer"))))

(when-not (fs/exists? cov-path)
  (die-usage! (str "no coverage export at " cov-path " — produce it with: "
                   "cargo llvm-cov nextest --workspace --json --output-path " cov-path)))

(def repo-root (str/trim (:out (sh "git" "rev-parse" "--show-toplevel"))))

(defn rel [p]
  (let [s (str p)]
    (when (str/starts-with? s (str repo-root "/"))
      (subs s (inc (count repo-root))))))

;; --- production .rs files: tracked, minus tests/benches/examples ------------
(def rust-files
  (->> (str/split-lines (:out (sh "git" "ls-files" "*.rs")))
       (remove str/blank?)
       (remove #(re-find #"(^|/)(tests|benches|examples)/" %))
       (remove #(re-find #"(^|/)build\.rs$" %))
       sort))

;; --- complexity: rust-code-analysis-cli per file, walk the spaces tree ------
(defn walk-spaces [node]
  (cons node (mapcat walk-spaces (:spaces node))))

(defn fns-in-file [f]
  (let [{:keys [exit out]} (sh "rust-code-analysis-cli" "-p" (str (fs/path repo-root f)) "-m" "-O" "json")]
    (when (and (zero? exit) (not (str/blank? out)))
      (for [space (walk-spaces (json/parse-string out true))
            :when (contains? #{"function" "closure"} (:kind space))
            :let [cyc (int (or (get-in space [:metrics :cyclomatic :sum]) 0))]
            :when (>= cyc 1)]
        {:file f :name (or (:name space) "<anon>")
         :start (:start_line space) :end (:end_line space)
         :cyc cyc :cog (int (or (get-in space [:metrics :cognitive :sum]) 0))}))))

(println (str "==> complexity: " (count rust-files) " files"))
(def functions (vec (mapcat fns-in-file rust-files)))

;; --- coverage: llvm-cov JSON regions, Kind 0 only, bucketed per rel file ----
(def regions-by-file
  (let [export (json/parse-string (slurp cov-path) true)]
    (->> (for [d (:data export)
               func (:functions d)
               :let [filenames (vec (:filenames func))]
               region (:regions func)
               :let [[ls _ le _ exec fid _ kind] region]
               :when (and (= 0 kind) (< fid (count filenames)))
               :let [rf (rel (filenames fid))]
               :when rf]
           [rf {:ls ls :le le :exec exec}])
         (reduce (fn [m [f r]] (update m f (fnil conj []) r)) {}))))

(defn coverage-of [{:keys [file start end]}]
  (let [overlapping (filter #(and (<= (:ls %) end) (>= (:le %) start))
                            (regions-by-file file))]
    (when (seq overlapping)
      (double (/ (count (filter #(pos? (:exec %)) overlapping))
                 (count overlapping))))))

;; Uninstrumented (no overlapping regions) scores pessimistically as 0.0
;; coverage but is counted separately — usually cfg-gated code the test
;; target never compiled, which is its own finding.
(def scored
  (vec (for [f functions
             :let [cov (coverage-of f)
                   c (double (or cov 0.0))]]
         (assoc f :cov cov
                :crap (+ (* (:cyc f) (:cyc f) (Math/pow (- 1.0 c) 3.0)) (:cyc f))))))

(def uninstrumented (count (filter #(nil? (:cov %)) scored)))
(def ranked (vec (sort-by :crap > scored)))
(def craps (mapv :crap scored))

(defn r2 [x] (format "%.2f" (double x)))
(defn median [xs] (let [s (vec (sort xs)) n (count s)]
                    (if (zero? n) 0.0 (nth s (quot n 2)))))
(def stats
  {:total (count scored)
   :mean (if (seq craps) (/ (reduce + craps) (count craps)) 0.0)
   :median (median craps)
   :max (if (seq craps) (apply max craps) 0.0)
   :mean-cov (let [cs (keep :cov scored)]
               (if (seq cs) (/ (reduce + cs) (count cs)) 0.0))})

;; --- markdown report (embeds its own triage playbook) -----------------------
(def worst (first ranked))
(def report-md
  (str "# CRAP report\n\n"
       "Functions: " (:total stats)
       " (uninstrumented, scored as 0% cov: " uninstrumented ")\n"
       "mean " (r2 (:mean stats)) " · median " (r2 (:median stats))
       " · max " (r2 (:max stats))
       " · mean coverage " (r2 (:mean-cov stats)) "\n\n"
       "For every function over threshold, in order:\n"
       "1. Split or simplify — reduce cyclomatic complexity.\n"
       "2. Cover the branches.\n"
       "3. If neither is feasible, report it as essential complexity —\n"
       "   the human decides. Don't chase the tail.\n"
       "NEVER: raise the threshold or pad coverage with weak tests —\n"
       "the mutation pass exposes padding anyway.\n\n"
       "| CRAP | cyc | cog | cov | function | location |\n"
       "|---|---|---|---|---|---|\n"
       (str/join "\n"
         (for [f (take top-n ranked)]
           (str "| " (r2 (:crap f)) " | " (:cyc f) " | " (:cog f) " | "
                (if (:cov f) (r2 (:cov f)) "—") " | `" (:name f) "` | "
                (:file f) ":" (:start f) " |")))
       "\n"))

(fs/create-dirs (fs/path repo-root "target/metrics"))
(spit (str (fs/path repo-root "target/metrics/crap-report.md")) report-md)

;; --- trend CSV: idempotent append keyed on (date, commit) -------------------
(def csv-path (str (fs/path repo-root "metrics/history.csv")))
(def header "date,commit,branch,total_functions,mean_crap,median_crap,max_crap,mean_coverage,worst_function,worst_file")
(def today (str (java.time.LocalDate/now)))
(def commit (str/trim (:out (sh "git" "rev-parse" "--short" "HEAD"))))
(def row (str/join "," [today commit
                        (str/trim (:out (sh "git" "rev-parse" "--abbrev-ref" "HEAD")))
                        (:total stats) (r2 (:mean stats)) (r2 (:median stats))
                        (r2 (:max stats)) (r2 (:mean-cov stats))
                        (:name worst) (str (:file worst) ":" (:start worst))]))
(fs/create-dirs (fs/parent csv-path))
(let [old (if (fs/exists? csv-path) (str/split-lines (slurp csv-path)) [])
      kept (remove #(str/starts-with? % (str today "," commit ",")) old)
      lines (if (seq kept) (concat kept [row]) [header row])]
  (spit csv-path (str (str/join "\n" lines) "\n")))

(println report-md)
(println (str "Report:  target/metrics/crap-report.md\nHistory: metrics/history.csv (commit it)"))

;; --- optional gate -----------------------------------------------------------
(when gate
  (let [over (filter #(> (:crap %) gate) ranked)]
    (when (seq over)
      (binding [*out* *err*]
        (println (str "\nCRAP GATE FAILED — " (count over) " function(s) over " gate ":"))
        (doseq [f over]
          (println (str "  " (r2 (:crap f)) "  " (:file f) ":" (:start f) "  " (:name f))))
        (println "Refactor (Extract Function, on green) or report as essential complexity."))
      (System/exit 1))))
