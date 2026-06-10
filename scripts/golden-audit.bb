#!/usr/bin/env bb
;; golden-audit.bb — bidirectional golden<->test linkage audit (golden-parity
;; skill). A golden nothing references is dead weight that still constrains
;; re-capture; a referenced golden missing from disk is the broken test that
;; must PANIC, not skip — this audit catches it before the suite does.
;;
;; Conventions: goldens under $GOLDEN_DIR (default "goldens", may be nested,
;; e.g. GOLDEN_DIR=crates/parity/goldens); tests/features reference a golden
;; by a path string containing "<dirname>/<relative-path>".
;; Checks:
;;   ORPHAN-GOLDEN   golden file no test/feature source references
;;   MISSING-GOLDEN  referenced path with no file on disk
;;   UNTRACKED       golden not tracked by git (capture ran, commit forgot)
;;   NO-BINARY-ATTR  goldens dir not covered by a `binary` .gitattributes
;;                   rule (git munges CRLF in goldens otherwise) — warning
;; Exit non-zero on ORPHAN/MISSING/UNTRACKED.

(require '[babashka.fs :as fs]
         '[babashka.process :refer [sh]]
         '[clojure.string :as str])

(def golden-dir (or (System/getenv "GOLDEN_DIR") "goldens"))

(when-not (fs/directory? golden-dir)
  (println (str "no " golden-dir "/ directory (set GOLDEN_DIR)"))
  (System/exit 1))

(def fail? (atom false))
(defn flag! [& parts] (println (str "  " (str/join parts))) (reset! fail? true))

(def dir-name (fs/file-name golden-dir))

;; golden files, as paths relative to the golden dir
(def goldens
  (->> (fs/glob golden-dir "**")
       (filter fs/regular-file?)
       (map #(str (fs/relativize golden-dir %)))
       sort))

;; every tracked source that could reference a golden
(def source-text
  (->> (str/split-lines (:out (sh "git" "ls-files"
                                  "*.rs" "*.feature" "*.toml" "*.bb" "*.sh")))
       (remove str/blank?)
       (filter fs/exists?)
       (map slurp)
       (str/join "\n")))

(println "== Orphan goldens (no test/feature references them) ==")
;; Tests often build golden paths ("goldens/blob_plain/" + name), invisible
;; to a per-file grep. So a golden counts as referenced if ANY prefix of its
;; path appears in the sources (file, or any ancestor directory). Trade-off,
;; documented: one directory mention vouches for the whole capture family —
;; orphans inside a referenced family go unseen here; `git diff` after
;; re-capture is what accounts for those (golden-parity skill).
(defn prefixes [rel]
  (let [parts (str/split rel #"/")]
    (map #(str/join "/" (take % parts)) (range 1 (inc (count parts))))))
(doseq [g goldens]
  (when-not (some (fn [p] (or (str/includes? source-text (str dir-name "/" p))
                              (str/includes? source-text p)))
                  (prefixes g))
    (flag! "ORPHAN-GOLDEN  " golden-dir "/" g)))

(println "== Missing goldens (referenced but not on disk) ==")
(doseq [ref (->> (re-seq (re-pattern (str "\\b" dir-name "/[A-Za-z0-9_./-]*[A-Za-z0-9_-]"))
                         source-text)
                 distinct sort)
        :let [rel (subs ref (inc (count dir-name)))
              on-disk (fs/path golden-dir rel)]]
  ;; a reference may name a subdirectory (capture family) — that's fine
  (when-not (fs/exists? on-disk)
    (flag! "MISSING-GOLDEN  " ref " (capture never ran, or path drifted)")))

(println "== Untracked goldens (captured but never committed) ==")
(let [untracked (->> (str/split-lines (:out (sh "git" "ls-files" "--others"
                                                "--exclude-standard" "--" golden-dir)))
                     (remove str/blank?))]
  (doseq [u untracked] (flag! "UNTRACKED  " u)))

(println "== .gitattributes binary rule ==")
(let [attrs (->> ["." (str (fs/parent golden-dir))]
                 (map #(fs/path % ".gitattributes"))
                 (filter fs/exists?)
                 (map (comp slurp str))
                 (str/join "\n"))]
  (when-not (and (str/includes? attrs dir-name) (str/includes? attrs "binary"))
    (println (str "  WARN no `" dir-name "/** binary` rule found — "
                  "git may munge line endings in goldens"))))

(if @fail?
  (do (println)
      (println (str "GOLDEN AUDIT FAILED — after any re-capture, `git diff " golden-dir
                    "/` and account for every changed byte (golden-parity skill)."))
      (System/exit 1))
  (do (println) (println "GOLDEN AUDIT OK.")))
