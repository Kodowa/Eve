receive the webhook from github on a pull request
  [#webhook-message
    source: "github.com"
    number
    repository: [git_url: github-url]
    pull_request: [body title additions deletions
                   state: "open"
                   head: [ref: branch, sha]]]
  update
    [#pull-request number, github-url, body, title, additions, deletions, branch, sha]

checkout the git repository
  [#pull-request github-url, branch, sha]
  path = "/tmp/{branch}/{sha}"
  update history
    [#git-repository #eve-repo source: github-url, branch, sha, path]

once checked out build eve
  [#success of: [#eve-repo path]]
  update history
    [#process #eve-build shell: "make", cwd: path]

once built, run eve
  [#success of: [#eve-build path]]
  [#eve-repo path sha]
  update history
    [#process #eve-run shell: "./eve -b {sha}", cwd: path, sha]

send facts over to that bag
  [#eve-repo sha]
  (entity, attribute, value) =
    if entity = [#test-inputs]
       [#eavs entity, attribute, value] then (entity, attribute, value)
    if entity = [#test-expected]
       [#eavs entity, attribute, value] then (entity, attribute, value)
    if entity = [#test-views]
       [#eavs entity, attribute, value] then (entity, attribute, value)
  update history sha
    [#eavs entity attribute value]

check that bag for the results
  [#eve-repo sha]
  context bag: sha
    [#test-result test result]
  end
  update history
    [#run sha result:
      [#test-result test result]]

display pull requests
  [#pull-request number, github-url, body, title, additions, deletions, branch, sha]
  update
    [#div children:
      [#header text: "{number}: {title}"]
      [#div text: "{branch} :: + {additions} / - {deletions}"]
      [#div text: body]
      [#div #result-display]]

display pending execution
  [#pull-request number, github-url, body, title, additions, deletions, branch, sha]
  not([#run sha])
  parent = [#result-display]
  update
    parent.children += [#div "running tests..."]

display test results
  [#pull-request number, github-url, body, title, additions, deletions, branch, sha]
  [#run sha]
  [#test-result test result]
  parent = [#result-display]
  update
    parent.children += [#div class: [success: result], children: [#span: test]]



