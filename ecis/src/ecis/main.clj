(ns ecis.main
  (:require
   [clojure.java.io :as io]
   [clojure.string :as string]
   [org.httpkit.server :as httpserver]
   [clj-jgit.porcelain :as porcelain]  
   [clj-json.core :as json]
   [gniazdo.core :as ws])
  (:import [java.io File BufferedWriter OutputStreamWriter BufferedReader InputStreamReader Reader]
           [org.apache.log4j BasicConfigurator Level Logger PropertyConfigurator]))


(def server "localhost:8081")
 
(defn delete-recursively [fname]
  (let [func (fn [func f]
               (when (.isDirectory f)
                 (doseq [f2 (.listFiles f)]
                   (func func f2)))
               (clojure.java.io/delete-file f))]
    (func func (clojure.java.io/file fname))))

(defn quotify [x] (str "\"" (string/replace (string/replace x "\n" "\\n") "\"" "\\\"") "\""))

(defn format-json [x]
  (condp #(%1 %2) x
    string? (quotify x)
    keyword? x ;;@NOTE: should this coerce to string?
    symbol? (quotify x)
    number? (if (integer? x)
              x
              (double x)) ;; @FIXME: This needs to be bigdec
    map? (str "{" (reduce-kv (fn [b k v] (str b (if (> (count b) 0) ", ") (format-json k) ":" (format-json v))) "" x) "}")
    coll? (str "[" (string/join "," (map format-json x)) "]")
    nil? "null"
    x))


(defn connect-to-eve [station user bag]
  (let [handlers (atom {})
        input #(let [j (json/parse-string %)
                     h (@handlers (symbol (j "id")))]
                 (when (and h (= (j "type") "result"))
                            (let [f (j "fields")
                                  ins (j "insert")
                                  rem (j "remove")]
                              (when (> (count ins) 0)
                                (h ins)))))
        target (str "ws://" station)
        ;; just bury any errors
        sock (try (ws/connect target :on-receive input)
                  (catch Exception e nil))]
    (if sock [sock handlers] sock)))
    


(defn eve-query [s q handler]
  (let [tag (gensym "q")
        q (format-json {"type" "query"
                        "query"  q
                        "id" tag})]
    (ws/send-msg (s 0) q)
    (swap! (s 1) assoc tag handler)
    [s tag]))
  

(defn eve-close [q]
  (let [m (format-json {"type" "close"
                             "id" (q 1)})]
    (ws/send-msg ((q 0) 0) m)))

  
(defn eve-insert [s eavs]
  (let [q (str (map #(str "(insert-fact! " 
                          (nth %1 0) " " 
                          (nth %1 1) " " 
                          (nth %1 2) ")")) eavs)]
    (eve-close (eve-query s q (fn [x] ())))))


(defn subprocess [path]
  (let [cmd ["/usr/local/bin/lein" "run" "-p" "8083"]
        proc (.exec (Runtime/getRuntime) 
                    ^"[Ljava.lang.String;" (into-array cmd)
                    nil ;; (into-array String [])
                    (File. path))
        out (new BufferedReader (new InputStreamReader (.getInputStream proc)))]
        
    (.start (Thread. (fn [] (println (.readLine out)))))
                       
    [(new BufferedWriter (new OutputStreamWriter (.getOutputStream proc))) (future (.waitFor proc))]))


(def charset (map char (concat (range 48 58) (range 66 92) (range 97 123))))

(defn checkout-repository [url branch]
  (let [pathname (apply str "/tmp/" (repeatedly 20 #(rand-nth charset)))]
       (porcelain/git-clone-full url pathname "origin" branch)
       pathname))

 
(defn tree-to-facts [m] 
  (let [facts (atom ())
        descend (fn descend [id m] 
                  (doseq [k (keys m)]
                    (let [v (m k)
                          f (fn [x] (swap! facts conj (list id (keyword k) x)))]
                      (if (= (type v) clojure.lang.PersistentArrayMap )
                        (let [sub (gensym "subflatto")]
                          (descend sub v)
                          (f sub))
                        (f v)))))]
    (descend (gensym "flatto") m)
    @facts))


(defn run-test [url branch facts]
  (let [path (checkout-repository url branch)
        s (atom nil)
        start "(load \"examples/harness.e\")\n"
        p (subprocess (str path "/server"))]
    (Thread/sleep 6000)
    (reset! s (connect-to-eve "localhost:8083" 0 0))
    (when (not @s)
      (Thread/sleep 3000)
      (reset! s (connect-to-eve "localhost:8083" 0 0)))

    (when @s
      (eve-query @s "(query [test success] (fact _ :tag \"test-run\" :result success :test))" 
                 (fn [x]
                   (let [database (connect-to-eve server 0 0)
                         results (atom {})
                         out (atom {})]
                     (doseq [i x]
                       (swap! results assoc-in [(keyword (first i)) :result] (second i))
                       (swap! results assoc-in [(keyword (first i)) :name] (first i)))
                     (println @results)
                     (doseq [i (keys @results)] 
                       (swap! out assoc :result (@results i)))
                     (swap! out merge facts)
                     (swap! out assoc :tag "test")
                     (let [fax (tree-to-facts @out)]
                       (println fax)
                       (eve-insert database fax)
                       (eve-close database))))))

    (.write (p 0) start)
    (.flush (p 0))
    (.write (p 0) "(exit)\n")
    (.flush (p 0))
    (println "test lein exit" @(p 1))
    (delete-recursively path)))
    
    
;; the websocket input guy   
(defn input-handler [request]
  (let [parsed (json/parsed-seq (clojure.java.io/reader (:body request) :encoding "UTF-8"))
        action (first parsed)
        repo ((action "repository") "git_url")
        pr (action "pull_request") 
        branch ((pr "head") "ref")
        state (pr "state")]
    (when (= state "open")
      (println "run test" repo branch)
      (let [user ((pr "user") "login")]
        (run-test repo branch {:user user})))
    {:body "thanks"}))

;; webhook input
(defn serve [port]
  (println (str "Serving on localhost:" port "/repl"))
  (try
    (httpserver/run-server input-handler {:port port})
    (catch Exception e (println (str "caught exception: " e (.getMessage e))))))

(defn -main [& args] 
  (org.apache.log4j.BasicConfigurator/configure) 
  (.setLevel (Logger/getRootLogger) Level/OFF)

;;  (let [k (connect-to-eve "127.0.0.1:8081" 0 0)]
;;    (eve-insert k "joeoy" :loves "salley"))

  (when (> (count args) 0) 
    (subprocess (first args)))
  (serve 8080))


