#!/usr/bin/env bb
;; mutation-run.bb — run cargo-mutants on the crates where mutation testing
;; makes sense, STREAMING the live caught/missed output to the console as it
;; happens, then score and SCREAM the verdict via mutation-report.bb.
;;
;; Composition mirror of quality-gate.bb -> crap-report.bb: this orchestrates
;; (runs the slow mutants + streams), mutation-report.bb scores. Default crate
;; set encodes WHERE mutation testing earns its keep (domain = business logic,
;; render = real formatting); glue/infra crates are deliberately absent.
;;
;; ALWAYS off the /tmp tmpfs (set here): cargo-mutants copies the whole target/
;; per mutant and a 16 GB tmpfs overflows -> every mutant falsely "unviable".
;;
;; Usage:
;;   mutation-run.bb [--shard k/n] [--gate N] [--archive DIR] [CRATE ...]
;;     CRATE       cargo package(s) (default: gitweb-domain gitweb-render)
;;     --shard k/n run only shard k of n (round-robin) — a tractable sample of
;;                 a huge crate; omit for a full run
;;     --gate N    fail (exit 1) if the final score is below N percent
;;     --archive   per-crate snapshot dir (default: target/metrics/mutation)
;; Each crate streams live, writes an isolated mutants.out under target/mutants/
;; <crate>, snapshots outcomes.json into the archive, and the aggregate is
;; scored + screamed at the end.

(require '[babashka.fs :as fs]
         '[babashka.process :as p :refer [shell sh]]
         '[clojure.java.io :as io]
         '[clojure.string :as str])

(def usage "usage: mutation-run.bb [--shard k/n] [--gate N] [--archive DIR] [CRATE ...]

Run cargo-mutants on the crates where mutation testing makes sense, streaming
live output, then score + scream via mutation-report.bb.

args:
  CRATE        cargo package(s) (default: gitweb-domain gitweb-render)
flags:
  --shard k/n  run only shard k of n (round-robin) — sample a huge crate
  --gate N     exit 1 if the aggregate score is below N percent
  --archive D  per-crate snapshot dir (default: target/metrics/mutation)
  -h, --help   this help

exit codes: 0 success | 1 gate not met | 2 usage/environment error
example: mutation-run.bb --shard 1/12 --gate 70 gitweb-domain")

(defn die-usage! [msg]
  (binding [*out* *err*] (println (str "error: " msg)) (println) (println usage))
  (System/exit 2))

(when (some #{"-h" "--help"} *command-line-args*)
  (println usage) (System/exit 0))

(def value-flags #{"--shard" "--gate" "--archive"})
(def parsed
  (loop [m {:crates []} [a & more] *command-line-args*]
    (cond
      (nil? a) m
      (value-flags a) (if (and (first more) (not (str/starts-with? (first more) "-")))
                        (recur (assoc m a (first more)) (rest more))
                        (die-usage! (str a " requires a value")))
      (str/starts-with? a "-") (die-usage! (str "unknown argument " a))
      :else (recur (update m :crates conj a) more))))

(def crates (if (seq (:crates parsed)) (:crates parsed)
                ["gitweb-domain" "gitweb-render"]))
(def shard (get parsed "--shard"))
(def gate (get parsed "--gate"))
(def archive (get parsed "--archive" "target/metrics/mutation"))

(when-not (fs/which "cargo-mutants")
  (die-usage! (str "cargo-mutants not installed. Install:\n"
                   "  cargo install --locked cargo-mutants\n"
                   "(verification-ratchet skill, Layer 3).")))

(def repo-root (str/trim (:out (sh "git" "rev-parse" "--show-toplevel"))))
(def tmp (str (fs/path (System/getenv "HOME") ".cache/cargo-mutants")))
(fs/create-dirs tmp)
(fs/create-dirs (fs/path repo-root archive))
(fs/create-dirs (fs/path repo-root "target/mutants"))

(def report (str (fs/path (fs/parent *file*) "mutation-report.bb")))

(def color? (and (nil? (System/getenv "NO_COLOR")) (some? (System/console))))
(defn paint [codes s] (if color? (str "[" codes "m" s "[0m") s))

;; A survivor is an issue to ACT ON now: re-emit cargo-mutants' MISSED line as a
;; loud, parsed block — location, the mutation that lived, and the only two
;; legal responses — so monitoring the run is actionable in real time.
(defn highlight-survivor [line]
  (if-let [[_ file ln mut] (re-find #"^\s*MISSED\s+(\S+?):(\d+):\d+:\s*(.*?)\s+in\s+\d" line)]
    (str/join "\n"
      [(str (paint "1;41" " ⚠ SURVIVOR ") "  " (paint "1;33" (str file ":" ln)))
       (str "             " mut)
       (str "             " (paint "2" "→ kill it (a test asserting the exact value) or report it"))])
    line))

;; Stream the subprocess line by line: echo everything (full progress), and the
;; instant a MISSED line appears, scream the parsed survivor under it. err is
;; inherited so cargo-mutants' live progress bar still shows.
(defn stream-mutants! [argv]
  (let [proc (p/process argv {:out :stream :err :inherit :extra-env {"TMPDIR" tmp}})]
    (with-open [rdr (io/reader (:out proc))]
      (doseq [line (line-seq rdr)]
        (println line)
        (when (re-find #"^\s*MISSED\s" line)
          (println (highlight-survivor line)))
        (flush)))
    (:exit @proc)))

;; --- run each crate, STREAMING live + highlighting survivors, then snapshot --
(doseq [crate crates]
  (let [short (str/replace crate #"^gitweb-" "")
        out-dir (str "target/mutants/" short)
        argv (vec (concat ["cargo" "mutants" "-p" crate "-o" out-dir]
                          (when shard ["--shard" shard])))]
    (println (str "\n==> mutants: " crate (when shard (str "  (shard " shard ")"))
                  "  — streaming live; survivors screamed inline"))
    (stream-mutants! argv)
    (let [outcomes (fs/path repo-root out-dir "mutants.out/outcomes.json")]
      (if (fs/exists? outcomes)
        (do (fs/copy outcomes (fs/path repo-root archive (str short ".json"))
                     {:replace-existing true})
            (println (str "    snapshot -> " archive "/" short ".json")))
        (binding [*out* *err*]
          (println (str "WARNING: no outcomes.json for " crate
                        " — run interrupted? skipping its snapshot.")))))))

;; --- score + SCREAM the aggregate -------------------------------------------
(println "\n==> scoring the aggregate")
(def report-args (concat ["bb" report "--archive" archive] (when gate ["--gate" gate])))
(System/exit (:exit (shell {:continue true} (str/join " " report-args))))
