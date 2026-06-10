#!/usr/bin/env bb
;; tracker-audit.bb — audit a beads issues.jsonl against tracker-discipline.
;; Works on the JSONL mirror (bd and br both emit it), no tracker CLI needed.

(require '[babashka.fs :as fs]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def usage "usage: tracker-audit.bb [--json] [issues.jsonl]

Audit a beads JSONL mirror against tracker-discipline.

args:
  issues.jsonl   path to the tracker's JSONL mirror
                 (default: .beads/issues.jsonl)
flags:
  --json         machine-readable result on stdout:
                 {\"ok\":bool,\"findings\":[{\"check\",\"detail\"}],\"warnings\":[...]}
  -h, --help     this help

checks (findings, exit 1):
  OPEN-EPIC-DONE     open parent, all dotted children (X.1, X.2…) closed —
                     the most common tracker rot
  DEFERRAL-NO-BEAD   close_reason defers without referencing a bead id
                     (accepts short id suffixes — \"2os.11\" for
                     \"proj-2os.11\"). Known false positive: \"deferred\"
                     as a placement word. Triage by hand.
  DANGLING-DEP       dependency edge pointing at a nonexistent id
warnings (never affect exit):
  STUB-OPEN          open bead with <200-char description — backlog
                     marker, not implementable; enrich before pickup

exit codes: 0 clean | 1 findings | 2 usage/environment error
example: tracker-audit.bb --json .beads/issues.jsonl")

;; --- args -------------------------------------------------------------------
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

(def known-flags #{"--json" "--help" "-h"})
(when (some #{"-h" "--help"} *command-line-args*)
  (println usage) (System/exit 0))
(doseq [a *command-line-args* :when (str/starts-with? a "-")]
  (when-not (known-flags a)
    (let [sug (first (sort-by #(lev a %) (filter #(<= (lev a %) 2) known-flags)))]
      (die-usage! (str "unknown flag " a (when sug (str " — did you mean " sug "?")))))))

(def json? (boolean (some #{"--json"} *command-line-args*)))
(def positional (remove #(str/starts-with? % "-") *command-line-args*))
(def path (or (first positional) ".beads/issues.jsonl"))

(when-not (fs/exists? path)
  (die-usage! (str "no such file: " path
                   " — pass the JSONL mirror path (bd/br emit it; check .beads/)")))

(def issues
  (->> (str/split-lines (slurp path))
       (remove str/blank?)
       (map #(json/parse-string % true))))

(def by-id (into {} (map (juxt :id identity)) issues))
(def ids (set (keys by-id)))
(def closed? #(= "closed" (:status %)))

;; --- collect ------------------------------------------------------------------
(def findings (atom []))
(def warnings (atom []))
(defn flag! [check & parts]
  (let [d (str/join parts)]
    (swap! findings conj {:check check :detail d})
    (when-not json? (println (str "  " check "  " d)))))
(defn warn! [check & parts]
  (let [d (str/join parts)]
    (swap! warnings conj {:check check :detail d})
    (when-not json? (println (str "  WARN " check "  " d)))))
(defn section! [title] (when-not json? (println (str "== " title " =="))))

(defn children-of [id]
  (let [prefix (str id ".")]
    (filter #(str/starts-with? (:id %) prefix) issues)))

(section! "Open epics with all children closed")
(doseq [issue issues
        :when (not (closed? issue))
        :let [kids (children-of (:id issue))]
        :when (and (seq kids) (every? closed? kids))]
  (flag! "OPEN-EPIC-DONE" (:id issue) "  (" (count kids)
         " children, all closed — close it or say what still blocks it)"))

(section! "Deferrals without a tracker home")
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
    (flag! "DEFERRAL-NO-BEAD" (:id issue)
           "  close_reason defers without referencing a bead id")))

(section! "Dangling dependency edges")
(doseq [issue issues
        dep (:dependencies issue)
        :let [target (:depends_on_id dep)]
        :when (and target (not (str/blank? target)) (not (ids target)))]
  (flag! "DANGLING-DEP" (:id issue) " -> " target " (" (:type dep) ")"))

(section! "Stub beads still open")
(doseq [issue issues
        :when (and (not (closed? issue))
                   (< (count (str (:description issue))) 200)
                   (empty? (children-of (:id issue))))]   ; epics carry detail in children
  (warn! "STUB-OPEN" (:id issue) "  \"" (:title issue)
         "\" (" (count (str (:description issue))) " chars — enrich before pickup)"))

;; --- render -------------------------------------------------------------------
(if json?
  (println (json/generate-string {:ok (empty? @findings)
                                  :findings @findings
                                  :warnings @warnings}))
  (if (seq @findings)
    (do (println) (println "TRACKER AUDIT FAILED — fix per tracker-discipline."))
    (do (println) (println "TRACKER AUDIT OK."))))
(System/exit (if (seq @findings) 1 0))
