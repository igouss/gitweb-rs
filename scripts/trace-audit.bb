#!/usr/bin/env bb
;; trace-audit.bb — bidirectional requirement<->test traceability audit.
;; Convention: spec files specs/REQ-<AREA>-<NNN>.md
;;             tests carry `// REQ-<AREA>-<NNN>` in the comment/attribute
;;             block directly above the test attribute.
;; Exit non-zero on any finding. Requires only babashka (https://babashka.org).

(require '[babashka.fs :as fs]
         '[clojure.string :as str])

(def usage "usage: trace-audit.bb

Bidirectional requirement<->test traceability audit. Run from the repo
root. No flags besides -h/--help; configuration is by convention.

env:
  SPEC_DIR   spec directory (default: specs)

checks: ORPHAN-REQ, DANGLING-REF, NO-REQ-ID, DANGLING-WIP, STALE-WIP,
        IN-PROGRESS-NO-WIP (see the traceability + slice-workflow skills)
exit codes: 0 clean | 1 findings | 2 usage/environment error
example: SPEC_DIR=specs trace-audit.bb")

(when (some #{"-h" "--help"} *command-line-args*)
  (println usage) (System/exit 0))
(when (seq *command-line-args*)
  (binding [*out* *err*]
    (println (str "error: unexpected argument " (first *command-line-args*)))
    (println) (println usage))
  (System/exit 2))

(def spec-dir (or (System/getenv "SPEC_DIR") "specs"))
(def test-paths ["src" "tests"])

(when-not (fs/directory? spec-dir)
  (binding [*out* *err*]
    (println (str "error: no " spec-dir "/ directory — run from the repo root "
                  "or set SPEC_DIR")))
  (System/exit 2))

(def fail? (atom false))
(defn flag! [& parts] (println (str "  " (str/join parts))) (reset! fail? true))

(def rust-files
  (->> test-paths
       (filter fs/exists?)
       (mapcat #(fs/glob % "**.rs"))
       (map str)
       sort))

(def file->lines
  (into {} (map (juxt identity #(vec (str/split-lines (slurp %)))) rust-files)))

(def all-test-text (str/join "\n" (map #(slurp %) rust-files)))

(def spec-files (sort (map str (fs/glob spec-dir "REQ-*.md"))))

(defn spec-id [f] (str/replace (fs/file-name f) #"\.md$" ""))

(defn spec-status [f]
  (some #(second (re-find #"^status:\s*(\S+)" %))
        (str/split-lines (slurp f))))

;; --- Direction 1: every requirement has at least one referencing test ------
(println "== Orphan requirements (no test references the ID) ==")
(doseq [f spec-files
        :let [id (spec-id f)]
        :when (not= "superseded" (spec-status f))]
  (when-not (re-find (re-pattern (str "//\\s*.*\\b" id "\\b")) all-test-text)
    (flag! "ORPHAN-REQ  " id "  (" f ")")))

;; --- Direction 2a: every REQ comment points at a real spec file ------------
(println "== Dangling test references (ID with no spec file) ==")
(doseq [id (->> (re-seq #"//\s*.*?(REQ-[A-Z]+-[0-9]+)" all-test-text)
                (map second) distinct sort)]
  (when-not (fs/exists? (fs/path spec-dir (str id ".md")))
    (flag! "DANGLING-REF  " id)))

;; --- Direction 2b: every test function carries a REQ comment ---------------
;; A test attribute must have a REQ ID somewhere in the contiguous block of
;; comment/attribute lines directly above it.
(println "== Tests with no REQ ID (pinning behavior nobody asked for) ==")
(def test-attr #"#\[(test|proptest|tokio::test|rstest)\b")
(doseq [[f lines] file->lines
        [i line] (map-indexed vector lines)
        :when (re-find test-attr line)]
  (let [above (->> (range (dec i) -1 -1)
                   (map #(str/trim (lines %)))
                   (take-while #(or (str/starts-with? % "//")
                                    (str/starts-with? % "#[")))
                   (str/join "\n"))]
    (when-not (re-find #"REQ-[A-Z]+-[0-9]+" above)
      (flag! "NO-REQ-ID  " f ":" (inc i)))))

;; --- Direction 3: WIP acceptance markers (slice-workflow skill) ------------
;; #[ignore = "WIP(REQ-X)"] is the only sanctioned ignore. It must reference
;; an existing spec with status: in-progress. STALE = spec accepted while the
;; acceptance test is still parked — the silent-weakening this audit exists for.
(println "== WIP acceptance markers (stale or dangling) ==")
(def wip-re #"#\[ignore = \"WIP\((REQ-[A-Z]+-[0-9]+)\)\"\]")
(doseq [[f lines] file->lines
        [i line] (map-indexed vector lines)
        :let [[_ id] (re-find wip-re line)]
        :when id
        :let [spec (str (fs/path spec-dir (str id ".md")))]]
  (cond
    (not (fs/exists? spec))
    (flag! "DANGLING-WIP  " id " at " f ":" (inc i) " (no spec file)")

    (not= "in-progress" (spec-status spec))
    (flag! "STALE-WIP  " id " at " f ":" (inc i)
           " (spec status is not in-progress)")))

;; Inverse: an in-progress slice spec must have its WIP acceptance test.
(doseq [f spec-files
        :when (= "in-progress" (spec-status f))
        :let [id (spec-id f)]]
  (when-not (re-find (re-pattern (str "WIP\\(" id "\\)")) all-test-text)
    (flag! "IN-PROGRESS-NO-WIP  " id
           " (no WIP acceptance test found — slice started without its outer loop?)")))

(if @fail?
  (do (println)
      (println "AUDIT FAILED — fix orphans per the traceability skill (do not delete checks).")
      (System/exit 1))
  (do (println)
      (println "AUDIT OK — all directions clean.")))
