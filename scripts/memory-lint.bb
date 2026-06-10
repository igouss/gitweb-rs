#!/usr/bin/env bb
;; memory-lint.bb — lint an agent memory corpus (memory-discipline skill).
;; The index is treated as DERIVED, not trusted: even a disciplined field
;; corpus shipped a dangling index entry pointing at a file never written.

(require '[babashka.fs :as fs]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def usage "usage: memory-lint.bb [--json] [memory-dir]

Lint an agent memory corpus (memory-discipline skill).

args:
  memory-dir   directory holding MEMORY.md + memory files
               (default: $MEMORY_DIR, else auto-derived from the cwd:
               ~/.claude/projects/<munged-cwd>/memory — the Claude Code
               memory dir for the project you're standing in)
flags:
  --json       machine-readable result on stdout:
               {\"ok\":bool,\"findings\":[{\"check\",\"detail\"}],\"warnings\":[...]}
  -h, --help   this help

checks (findings, exit 1):
  DANGLING-WIKILINK   [[link]] with no <link>.md in the directory
  DANGLING-INDEX      MEMORY.md links a file that doesn't exist
  NO-INDEX            memories exist but MEMORY.md missing
  UNINDEXED           memory file absent from MEMORY.md
  NO-FRONTMATTER      missing name:/description: frontmatter
warnings (never affect exit):
  OVERSIZED           file > 10KB — split by sub-topic

exit codes: 0 clean | 1 findings | 2 usage/environment error
example: memory-lint.bb --json ~/.claude/projects/<proj>/memory/")

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

(defn project-memory-dir []
  ;; Claude Code's per-project dir: cwd munged with [/_.] -> "-"
  (let [munged (str/replace (System/getProperty "user.dir") #"[/_.]" "-")
        d (str (System/getProperty "user.home") "/.claude/projects/" munged "/memory")]
    (when (fs/directory? d)
      (binding [*out* *err*]
        (println (str "note: auto-detected memory dir: " d)))
      d)))

(def mem-dir (or (first positional) (System/getenv "MEMORY_DIR") (project-memory-dir)))

(when-not (and mem-dir (fs/directory? mem-dir))
  (die-usage! (str "no memory directory found (no arg, no MEMORY_DIR, and no "
                   "~/.claude/projects/<this-project>/memory exists).\n"
                   "Memories are written by agent sessions — for what to save and in\n"
                   "what shape (genres, frontmatter, MEMORY.md index) see the\n"
                   "memory-discipline skill.")))

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

(def md-files (->> (fs/list-dir mem-dir "*.md") (map str) sort))

;; Vacuous-green guard: an empty corpus audits nothing.
(when (empty? md-files)
  (die-usage! (str mem-dir " contains no .md files — nothing to lint.\n"
                   "An agent memory corpus is MEMORY.md (index) plus one file per\n"
                   "memory with name:/description: frontmatter — memory-discipline skill.")))
(def memories (remove #(= "MEMORY.md" (fs/file-name %)) md-files))
(def index-file (str (fs/path mem-dir "MEMORY.md")))
(def index-text (if (fs/exists? index-file) (slurp index-file) ""))

(defn exists-as-memory? [name]
  (fs/exists? (fs/path mem-dir (str name ".md"))))

(section! "Dangling wiki-links")
(doseq [f md-files
        link (->> (re-seq #"\[\[([^\]]+)\]\]" (slurp f)) (map second) distinct)]
  (when-not (exists-as-memory? link)
    (flag! "DANGLING-WIKILINK" "[[" link "]] in " (fs/file-name f))))

(section! "Index entries (MEMORY.md)")
(if-not (fs/exists? index-file)
  (when (seq memories) (flag! "NO-INDEX" "MEMORY.md missing"))
  (doseq [target (->> (re-seq #"\]\(([^)]+\.md)\)" index-text) (map second) distinct)]
    (when-not (fs/exists? (fs/path mem-dir target))
      (flag! "DANGLING-INDEX" target " linked in MEMORY.md, no such file"))))

(section! "Unindexed memories")
(doseq [f memories
        :let [n (fs/file-name f)]]
  (when-not (str/includes? index-text n)
    (flag! "UNINDEXED" n " (add a one-line entry to MEMORY.md)")))

(section! "Frontmatter")
(doseq [f memories
        :let [head (->> (str/split-lines (slurp f)) (take 12) (str/join "\n"))]]
  (when-not (and (re-find #"(?m)^name:" head) (re-find #"(?m)^description:" head))
    (flag! "NO-FRONTMATTER" (fs/file-name f) " (needs name: + description:)")))

(section! "Size")
(doseq [f memories
        :let [kb (quot (fs/size f) 1024)]]
  (when (> kb 10)
    (warn! "OVERSIZED" (fs/file-name f) " (" kb "KB) — split by sub-topic")))

;; --- render -------------------------------------------------------------------
(if json?
  (println (json/generate-string {:ok (empty? @findings)
                                  :findings @findings
                                  :warnings @warnings}))
  (if (seq @findings)
    (do (println)
        (println "MEMORY LINT FAILED — fix per memory-discipline (index is derived, not trusted)."))
    (do (println) (println "MEMORY LINT OK."))))
(System/exit (if (seq @findings) 1 0))
