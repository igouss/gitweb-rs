#!/usr/bin/env bb
;; tracker-audit.bb — audit a beads issues.jsonl against tracker-discipline.
;; Works on the JSONL mirror (bd and br both emit it), no tracker CLI needed.
;;
;; Usage: tracker-audit.bb [path/to/issues.jsonl]  (default .beads/issues.jsonl)
;; Checks:
;;   OPEN-EPIC-DONE     open parent whose children (dotted ids: X.1, X.2…)
;;                      are all closed — the most common tracker rot; closing
;;                      the epic when the last child lands is an explicit habit
;;   DEFERRAL-NO-BEAD   close_reason says deferred/out of scope/follow-up but
;;                      references no other bead id — a deferral with no
;;                      tracker home rots silently (the no-exceptions rule).
;;                      Known false-positive mode: "deferred" as a placement
;;                      word ("deferred to the render adapter"). Triage by
;;                      hand; field rate was ~half real, half word-usage.
;;   DANGLING-DEP       dependency edge pointing at a nonexistent id
;;   WARN STUB-OPEN     open bead with a <200-char description — fine as a
;;                      backlog marker, NOT implementable; enrich before pickup
;; Exit non-zero on any non-WARN finding.

(require '[babashka.fs :as fs]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def path (or (first *command-line-args*) ".beads/issues.jsonl"))

(when-not (fs/exists? path)
  (println (str "no such file: " path))
  (System/exit 1))

(def issues
  (->> (str/split-lines (slurp path))
       (remove str/blank?)
       (map #(json/parse-string % true))))

(def by-id (into {} (map (juxt :id identity)) issues))
(def ids (set (keys by-id)))
(def closed? #(= "closed" (:status %)))

(def fail? (atom false))
(defn flag! [& parts] (println (str "  " (str/join parts))) (reset! fail? true))
(defn warn! [& parts] (println (str "  WARN " (str/join parts))))

(defn children-of [id]
  (let [prefix (str id ".")]
    (filter #(str/starts-with? (:id %) prefix) issues)))

(println "== Open epics with all children closed ==")
(doseq [issue issues
        :when (not (closed? issue))
        :let [kids (children-of (:id issue))]
        :when (and (seq kids) (every? closed? kids))]
  (flag! "OPEN-EPIC-DONE  " (:id issue) "  (" (count kids)
         " children, all closed — close it or say what still blocks it)"))

(println "== Deferrals without a tracker home ==")
;; Close reasons commonly cite SHORT id suffixes ("Follow-ups filed: 2os.11")
;; rather than full ids — accept both. The short form is the full id minus
;; the common project prefix shared by every id in the file.
(def id-prefix
  (let [common (reduce (fn [a b]
                         (->> (map vector a b)
                              (take-while (fn [[x y]] (= x y)))
                              (map first) (apply str)))
                       (vec ids))]
    (if-let [i (str/last-index-of common "-")] (subs common 0 (inc i)) "")))
(defn short-id [id]
  (if (and (seq id-prefix) (str/starts-with? id id-prefix))
    (subs id (count id-prefix)) id))
(defn mentions-id? [text id]
  (boolean (re-find (re-pattern (str "(^|[^A-Za-z0-9.])"
                                     (java.util.regex.Pattern/quote (short-id id))
                                     "($|[^A-Za-z0-9.])"))
                    text)))
(def deferral-re #"(?i)\bdefer|out of scope|follow[- ]?up")
(doseq [issue issues
        :let [reason (str (:close_reason issue))]
        :when (and (closed? issue) (re-find deferral-re reason))]
  ;; a deferral is homed if the close reason names any OTHER existing bead id
  (when-not (some #(and (not= % (:id issue)) (mentions-id? reason %)) ids)
    (flag! "DEFERRAL-NO-BEAD  " (:id issue)
           "  close_reason defers without referencing a bead id")))

(println "== Dangling dependency edges ==")
(doseq [issue issues
        dep (:dependencies issue)
        :let [target (:depends_on_id dep)]
        :when (and target (not (str/blank? target)) (not (ids target)))]
  (flag! "DANGLING-DEP  " (:id issue) " -> " target " (" (:type dep) ")"))

(println "== Stub beads still open ==")
(doseq [issue issues
        :when (and (not (closed? issue))
                   (< (count (str (:description issue))) 200)
                   (empty? (children-of (:id issue))))]   ; epics carry detail in children
  (warn! "STUB-OPEN  " (:id issue) "  \"" (:title issue)
         "\" (" (count (str (:description issue))) " chars — enrich before pickup)"))

(if @fail?
  (do (println)
      (println "TRACKER AUDIT FAILED — fix per tracker-discipline.")
      (System/exit 1))
  (do (println) (println "TRACKER AUDIT OK.")))
