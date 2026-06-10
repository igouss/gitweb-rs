#!/usr/bin/env bb
;; trace-audit.bb — bidirectional requirement<->test traceability audit.
;; Convention: spec files specs/REQ-<AREA>-<NNN>.md
;;             tests carry `// REQ-<AREA>-<NNN>` in the comment/attribute
;;             block directly above the test attribute.
;; Exit non-zero on any finding. Requires only babashka (https://babashka.org).

(require '[babashka.fs :as fs]
         '[babashka.process :refer [sh]]
         '[clojure.string :as str])

(def usage "usage: trace-audit.bb

Bidirectional requirement<->test traceability audit. Run from the repo
root. No flags besides -h/--help; configuration is by convention.

By default scans every git-tracked .rs file — layout-agnostic (single
crate or cargo workspace, src/ or crates/, no configuration needed).

env:
  SPEC_DIR     spec directory (default: specs)
  TEST_PATHS   comma-separated dirs to scan instead of the git index
               (escape hatch for non-git trees or scoping)
  REQ_ID_CHECKS run the Mode A REQ-ID/spec audit (default: OFF — this is a
               Mode B project; .feature files are the spec, no REQ IDs)

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

;; REQ-ID / spec traceability (specs/REQ-<AREA>-<NNN>.md + `// REQ-...` test
;; comments) is a Mode A feature. This is a Mode B project: the .feature files
;; ARE the spec and there are no REQ IDs (see .claude/settings.json — the Stop
;; hook already omits this audit for the same reason). So every direction below
;; is N/A here and these checks are DISABLED BY DEFAULT (human-approved
;; check-change). Opt back into full Mode A auditing with REQ_ID_CHECKS=1 — e.g.
;; if this tree ever adopts specs/REQ-*.md files.
(def req-id-checks?
  (contains? #{"1" "true" "yes" "on"}
             (str/lower-case (or (System/getenv "REQ_ID_CHECKS") ""))))

(when-not req-id-checks?
  (println "REQ-ID / spec traceability checks are DISABLED BY DEFAULT (Mode B:")
  (println ".feature files are the spec; this repo carries no REQ IDs).")
  (println "Set REQ_ID_CHECKS=1 to run the full Mode A audit. Nothing to do.")
  (System/exit 0))

(def spec-dir (or (System/getenv "SPEC_DIR") "specs"))

(when-not (fs/directory? spec-dir)
  (binding [*out* *err*]
    (println (str "error: no " spec-dir "/ directory — nothing to audit."))
    (println "Run from the repo root or set SPEC_DIR. To start Mode A spec-driven")
    (println (str "work here: mkdir -p " spec-dir " && copy the kit's"))
    (println (str "templates/requirement.md to " spec-dir "/_template.md, then write one"))
    (println (str spec-dir "/REQ-<AREA>-<NNN>.md per requirement — see the spec-authoring"))
    (println "skill. Mode B projects (.feature files ARE the spec) don't use this audit."))
  (System/exit 2))

(def fail? (atom false))
(defn flag! [& parts] (println (str "  " (str/join parts))) (reset! fail? true))

(def rust-files
  (if-let [tp (System/getenv "TEST_PATHS")]
    (->> (str/split tp #",")
         (filter fs/exists?)
         (mapcat #(fs/glob % "**.rs"))
         (map str)
         sort)
    ;; default: the git index decides the layout — works for src/, crates/,
    ;; or anything else, with zero configuration
    (->> (str/split-lines (:out (sh "git" "ls-files" "*.rs")))
         (remove str/blank?)
         (filter fs/exists?)
         sort)))

;; Vacuous-green guard (quality-gates skill): an audit that scanned zero
;; files proves nothing — refuse to report OK over an empty input set.
(when (empty? rust-files)
  (binding [*out* *err*]
    (println "error: no .rs files found — nothing to audit.")
    (println "Run from the repo root of a git-tracked Rust project, or set")
    (println "TEST_PATHS=dir1,dir2 for a non-git tree. See the traceability")
    (println "skill for the REQ-ID convention this audits."))
  (System/exit 2))

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
