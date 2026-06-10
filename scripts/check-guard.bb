#!/usr/bin/env bb
;; check-guard.bb — mechanical detector for check-weakening in a diff.
;; Catches what cargo-mutants --in-diff structurally cannot: a diff touching
;; only TEST code runs zero mutants, so test-gutting is invisible to
;; incremental mutation testing. This script parses the diff itself.
;;
;; Modes:
;;   check-guard.bb                 ; working tree vs HEAD (strict)
;;   check-guard.bb --staged       ; staged changes (pre-commit hook, strict)
;;   check-guard.bb --range A..B   ; CI: per-commit; commits whose subject
;;                                 ;   starts with "check-change:" are exempt
;;                                 ;   from W-rules (the label is the
;;                                 ;   human-approved escape hatch — your
;;                                 ;   review of labeled commits is the gate)
;;
;; Local workflow for an APPROVED check change: commit it standalone with the
;; check-change: scope; if a strict-mode pre-commit hook blocks it, commit
;; with --no-verify — CI range mode validates the label.
;;
;; Findings are HEURISTIC: a pass means "no obvious weakening", not proof —
;; cargo-mutants remains the proof. Requires babashka + git.

(require '[babashka.process :refer [shell]]
         '[clojure.string :as str])

(defn git [& a]
  (-> (apply shell {:out :string :err :string :continue true} "git" a) :out))

;; --- structured diff: seq of {:file f :added [lines] :deleted [lines]} -----
(defn parse-diff [text]
  (->> (str/split-lines text)
       (reduce (fn [{:keys [cur] :as acc} line]
                 (cond
                   (str/starts-with? line "diff --git")
                   (let [f (-> line (str/split #" ") (nth 2)
                               (str/replace-first #"^a/" ""))]
                     (-> acc
                         (update :files (fnil conj []) cur)
                         (assoc :cur {:file f :added [] :deleted []})))

                   (and cur (str/starts-with? line "+")
                        (not (str/starts-with? line "+++")))
                   (update-in acc [:cur :added] conj (subs line 1))

                   (and cur (str/starts-with? line "-")
                        (not (str/starts-with? line "---")))
                   (update-in acc [:cur :deleted] conj (subs line 1))

                   :else acc))
               {:cur nil :files []})
       ((fn [{:keys [files cur]}] (remove nil? (conj files cur))))))

(defn test-file? [f]
  (or (re-find #"(^|/)tests/" f)
      (re-find #"test" f)
      (re-find #"proptest-regressions/" f)))

(def wip-ok #"#\[ignore = \"WIP\(REQ-[A-Z]+-[0-9]+\)\"\]")
(def infra-re
  #"(\.cargo[-/]mutants\.toml|mutants\.toml|quality-gate\.bb|trace-audit\.bb|check-guard\.bb|protect-checks-hook\.bb|hex-lint-exceptions\.toml|\.claude/settings\.json)")

;; W-rules over a parsed diff -> vector of finding strings.
(defn scan [hunks]
  (vec
   (concat
    ;; W1: test-disabling attributes added. Sanctioned exception
    ;; (slice-workflow skill): the exact WIP(REQ-...) ignore form.
    ;; Staleness of WIP markers is trace-audit's job.
    (for [{:keys [file added]} hunks
          line added
          :when (and (re-find #"#\[(ignore|mutants::skip)" line)
                     (not (re-find wip-ok line)))]
      (str "W1-disable] " file ": +" (str/trim line)))
    ;; W2: assertions deleted from test code.
    (for [{:keys [file deleted]} hunks
          :when (test-file? file)
          line deleted
          :when (re-find #"assert" line)]
      (str "W2-assert-del] " file ": -" (str/trim line)))
    ;; W3: proptest regression seeds deleted/modified (append-only archives).
    (for [{:keys [file deleted]} hunks
          :when (re-find #"proptest-regressions/" file)
          line deleted]
      (str "W3-regression-del] " file ": -" (str/trim line)))
    ;; W4: check infrastructure touched.
    (for [{:keys [file]} hunks
          :when (re-find infra-re file)]
      (str "W4-config] check infrastructure modified: " file)))))

(def args *command-line-args*)
(def mode (or (first args) "--worktree"))
(def range-spec (second args))

(def findings (atom []))
(defn add! [fs] (swap! findings into fs))

(println "== check-guard: scanning for check-weakening patterns ==")

(case mode
  "--range"
  (do
    (when-not range-spec
      (binding [*out* *err*] (println "usage: check-guard.bb --range A..B"))
      (System/exit 1))
    (doseq [c (->> (git "rev-list" range-spec) str/split-lines
                   (remove str/blank?))]
      (let [subj (str/trim (git "log" "-1" "--format=%s" c))
            exempt? (str/starts-with? subj "check-change:")
            hunks (parse-diff (git "diff" (str c "^.." c) "--unified=0"))]
        (if exempt?
          (println (str "  exempt (check-change): " (subs c 0 8) " '" subj "'"))
          (do
            ;; W-rules per unlabeled commit
            (add! (map #(str % "  [commit " (subs c 0 8) "]") (scan hunks)))
            ;; T1: unlabeled commit deleting test lines (broader than W2:
            ;; any deletion in test paths, not just asserts)
            (when (some (fn [{:keys [file deleted]}]
                          (and (test-file? file) (seq deleted)))
                        hunks)
              (add! [(str "T1-unlabeled] commit " (subs c 0 8) " ('" subj
                          "') deletes test lines without check-change: scope")])))))))

  ;; strict single-diff modes
  (let [diff-text (case mode
                    "--staged" (git "diff" "--cached" "--unified=0")
                    (git "diff" "HEAD" "--unified=0"))]
    (if (str/blank? diff-text)
      (println "  empty diff")
      (add! (scan (parse-diff diff-text))))))

(if (seq @findings)
  (do
    (doseq [f @findings] (println (str "  WEAKENING[" f)))
    (println)
    (println "CHECK-GUARD FAILED.")
    (println "Per CLAUDE.md: checks may not be weakened to make work pass.")
    (println "If a check change is genuinely intended, it must be a standalone,")
    (println "human-approved commit with subject scope 'check-change:'.")
    (System/exit 1))
  (println "check-guard: OK (heuristic pass — mutation testing remains the proof)"))
