local compiler = require "compiler"

compiler.compileExec(
[[
Add some fruits to the system
  update
    [#fruit @lemon color: "yellow"]
    [#fruit @cherry color: "red"]
    [#fruit @orange color: "orange"]
    [#fruit @apple color: "green"]
    [#fruit @banana color: "yellow"]
    [#fruit #exotic @lychee color: "white"]
  end

Draw a list of fruits
  [#fruit color name]
  update session
    [#div text: name, style: [color] ]
  end
]],
function(executor) 
  run(executor)
end)

