#!/usr/bin/env bb
;; memory-lint.bb — lint an agent memory corpus (memory-discipline skill).
;; The index is treated as DERIVED, not trusted: even a disciplined field
;; corpus shipped a dangling index entry pointing at a file never written.
;;
;; Usage: memory-lint.bb [memory-dir]   (default: MEMORY_DIR env or ".")
;; Checks:
;;   DANGLING-WIKILINK   [[link]] with no <link>.md in the directory
;;   DANGLING-INDEX      MEMORY.md links a file that doesn't exist
;;   UNINDEXED           memory file absent from MEMORY.md (index is the
;;                       routing table — an unlisted memory is unreachable)
;;   NO-FRONTMATTER      missing name:/description: (description is
;;                       load-bearing: recall decisions are made on it)
;;   WARN OVERSIZED      file > 10KB — section or split before it accretes
;;                       (a field gotcha file hit 28KB)
;; Exit non-zero on any non-WARN finding.

(require '[babashka.fs :as fs]
         '[clojure.string :as str])

(def mem-dir (or (first *command-line-args*) (System/getenv "MEMORY_DIR") "."))

(when-not (fs/directory? mem-dir)
  (println (str "no such directory: " mem-dir))
  (System/exit 1))

(def fail? (atom false))
(defn flag! [& parts] (println (str "  " (str/join parts))) (reset! fail? true))
(defn warn! [& parts] (println (str "  WARN " (str/join parts))))

(def md-files
  (->> (fs/list-dir mem-dir "*.md") (map str) sort))
(def memories (remove #(= "MEMORY.md" (fs/file-name %)) md-files))
(def index-file (str (fs/path mem-dir "MEMORY.md")))
(def index-text (if (fs/exists? index-file) (slurp index-file) ""))

(defn exists-as-memory? [name]
  (fs/exists? (fs/path mem-dir (str name ".md"))))

(println "== Dangling wiki-links ==")
(doseq [f md-files
        link (->> (re-seq #"\[\[([^\]]+)\]\]" (slurp f)) (map second) distinct)]
  (when-not (exists-as-memory? link)
    (flag! "DANGLING-WIKILINK  [[" link "]] in " (fs/file-name f))))

(println "== Index entries (MEMORY.md) ==")
(if-not (fs/exists? index-file)
  (when (seq memories) (flag! "NO-INDEX  MEMORY.md missing"))
  (doseq [target (->> (re-seq #"\]\(([^)]+\.md)\)" index-text) (map second) distinct)]
    (when-not (fs/exists? (fs/path mem-dir target))
      (flag! "DANGLING-INDEX  " target " linked in MEMORY.md, no such file"))))

(println "== Unindexed memories ==")
(doseq [f memories
        :let [n (fs/file-name f)]]
  (when-not (str/includes? index-text n)
    (flag! "UNINDEXED  " n " (add a one-line entry to MEMORY.md)")))

(println "== Frontmatter ==")
(doseq [f memories
        :let [head (->> (str/split-lines (slurp f)) (take 12) (str/join "\n"))]]
  (when-not (and (re-find #"(?m)^name:" head) (re-find #"(?m)^description:" head))
    (flag! "NO-FRONTMATTER  " (fs/file-name f) " (needs name: + description:)")))

(println "== Size ==")
(doseq [f memories
        :let [kb (quot (fs/size f) 1024)]]
  (when (> kb 10)
    (warn! "OVERSIZED  " (fs/file-name f) " (" kb "KB) — split by sub-topic")))

(if @fail?
  (do (println)
      (println "MEMORY LINT FAILED — fix per memory-discipline (index is derived, not trusted).")
      (System/exit 1))
  (do (println) (println "MEMORY LINT OK.")))
