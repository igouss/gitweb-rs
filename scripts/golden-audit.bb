#!/usr/bin/env bb
;; golden-audit.bb — bidirectional golden<->test linkage audit (golden-parity
;; skill). A golden nothing references is dead weight that still constrains
;; re-capture; a referenced golden missing from disk is the broken test that
;; must PANIC, not skip — this audit catches it before the suite does.

(require '[babashka.fs :as fs]
         '[babashka.process :refer [sh]]
         '[cheshire.core :as json]
         '[clojure.string :as str])

(def usage "usage: golden-audit.bb [--json] [golden-dir]

Bidirectional golden<->test linkage audit (golden-parity skill).
Run from the repo root, after wiring a new endpoint or any re-capture.

args:
  golden-dir   goldens directory, may be nested (e.g. crates/parity/goldens)
               (default: $GOLDEN_DIR, else \"goldens\")
flags:
  --json       machine-readable result on stdout:
               {\"ok\":bool,\"findings\":[{\"check\",\"detail\"}],\"warnings\":[...]}
  -h, --help   this help

checks (findings, exit 1):
  ORPHAN-GOLDEN    golden file no test/feature source references. A golden
                   counts as referenced if ANY prefix of its path appears
                   (one directory mention vouches for the capture family —
                   constructed paths are invisible to grep).
  MISSING-GOLDEN   referenced path with no file on disk (capture never ran,
                   or path drifted) — the must-panic case, caught early
  UNTRACKED        golden not tracked by git (capture ran, commit forgot)
warnings (never affect exit):
  NO-BINARY-ATTR   no `<dir>/** binary` .gitattributes rule — git may
                   munge line endings in goldens

exit codes: 0 clean | 1 findings | 2 usage/environment error
example: GOLDEN_DIR=crates/parity/goldens golden-audit.bb --json")

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
(def golden-dir (or (first positional) (System/getenv "GOLDEN_DIR") "goldens"))

(when-not (fs/directory? golden-dir)
  (die-usage! (str "no " golden-dir "/ directory — pass the goldens dir as the "
                   "first arg or set GOLDEN_DIR (run from the repo root)")))

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

(section! "Orphan goldens (no test/feature references them)")
(defn prefixes [rel]
  (let [parts (str/split rel #"/")]
    (map #(str/join "/" (take % parts)) (range 1 (inc (count parts))))))
(doseq [g goldens]
  (when-not (some (fn [p] (or (str/includes? source-text (str dir-name "/" p))
                              (str/includes? source-text p)))
                  (prefixes g))
    (flag! "ORPHAN-GOLDEN" golden-dir "/" g)))

(section! "Missing goldens (referenced but not on disk)")
(doseq [ref (->> (re-seq (re-pattern (str "\\b" dir-name "/[A-Za-z0-9_./-]*[A-Za-z0-9_-]"))
                         source-text)
                 distinct sort)
        :let [rel (subs ref (inc (count dir-name)))
              on-disk (fs/path golden-dir rel)]]
  ;; a reference may name a subdirectory (capture family) — that's fine
  (when-not (fs/exists? on-disk)
    (flag! "MISSING-GOLDEN" ref " (capture never ran, or path drifted)")))

(section! "Untracked goldens (captured but never committed)")
(let [untracked (->> (str/split-lines (:out (sh "git" "ls-files" "--others"
                                                "--exclude-standard" "--" golden-dir)))
                     (remove str/blank?))]
  (doseq [u untracked] (flag! "UNTRACKED" u)))

(section! ".gitattributes binary rule")
(let [attrs (->> ["." (str (fs/parent golden-dir))]
                 (map #(fs/path % ".gitattributes"))
                 (filter fs/exists?)
                 (map (comp slurp str))
                 (str/join "\n"))]
  (when-not (and (str/includes? attrs dir-name) (str/includes? attrs "binary"))
    (warn! "NO-BINARY-ATTR" "no `" dir-name "/** binary` rule found — "
           "git may munge line endings in goldens")))

;; --- render -------------------------------------------------------------------
(if json?
  (println (json/generate-string {:ok (empty? @findings)
                                  :findings @findings
                                  :warnings @warnings}))
  (if (seq @findings)
    (do (println)
        (println (str "GOLDEN AUDIT FAILED — after any re-capture, `git diff " golden-dir
                      "/` and account for every changed byte (golden-parity skill).")))
    (do (println) (println "GOLDEN AUDIT OK."))))
(System/exit (if (seq @findings) 1 0))
