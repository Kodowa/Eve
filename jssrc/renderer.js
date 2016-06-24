"use strict"

//---------------------------------------------------------
// MicroReact renderer
//---------------------------------------------------------

let renderer = new Renderer();
document.body.appendChild(renderer.content);

//---------------------------------------------------------
// handle dom updates
//---------------------------------------------------------

// TODO: queue updates to be applied during requestAnimationFrame

var activeElements = {"root": document.createElement("div")};
var activeStyles = {};
var supportedTags = {"div": true, "span": true, "input": true};

function insertSorted(parent, child) {
  let current;
  for(let curIx = 0; curIx < parent.children.length; curIx++) {
    current = parent.children[curIx];
    if(current.sort && current.sort > child.sort) {
      break;
    } else {
      current = null;
    }
  }
  if(current) {
    parent.insertBefore(child, current);
  } else  {
    parent.appendChild(child);
  }
}

function safeEav(eav) {
  if(eav[0].type == "uuid")  {
    eav[0] = `⦑${eav[0].value}⦒`
  }
  if(eav[1].type == "uuid")  {
    eav[1] = `⦑${eav[1].value}⦒`
  }
  if(eav[2].type == "uuid")  {
    eav[2] = `⦑${eav[2].value}⦒`
  }
  return eav;
}

function handleDOMUpdates(result) {
  let {insert, remove} = result;
  let additions = {};
  // build up a representation of the additions
  if(insert.length) {
    for(let ins of insert) {
      let [entity, attribute, value] = safeEav(ins);
      if(!additions[entity]) additions[entity] = {}
      switch(attribute) {
        case "tag":
          // we don't care about tags on this guy unless they relate
          // to dom tags
          if(!supportedTags[value]) {
            continue;
          }
          break;
        case "children":
          let children = additions[entity][attribute];
          if(!children) {
            children = [];
            additions[entity][attribute] = children;
          }
          children.push(value);
          continue;
        case "text":
          attribute = "textContent"
          break;
      }
      additions[entity][attribute] = value
    }
  }
  // do removes that aren't just going to be overwritten by
  // the adds
  if(remove && remove.length) {
    // we clean up styles after the fact so that in the case where
    // the style object is being removed, but the element is sticking
    // around, we remove any styles that may have been applied
    let stylesToGC = [];
    for(let rem of remove) {
      let [entity, attribute, value] = safeEav(rem);
      if(activeStyles[entity]) {
        // do style stuff
        let style = activeStyles[entity].style;
        if(!additions[entity] || !additions[entity][attribute]) {
          style[attribute] = "";
        }
      } else if(activeElements[entity]) {
        let elem = activeElements[entity];
        switch(attribute) {
          case "tag":
            if(supportedTags[value]) {
              //nuke the whole element
              elem.parentNode.removeChild(elem);
              activeElements[entity] = null;
            }
            break;
          case "style":
            stylesToGC.push(value);
            break;
          case "children":
            let child = activeElements[value];
            if(child) {
              elem.removeChild(child);
              activeElements[value] = null;
            }
            break;
          case "text":
            if(!additions[entity] || !additions[entity]["text"]) {
              elem.textContent = "";
            }
            break;
          default:
            if(!additions[entity] || !additions[entity][attribute]) {
              //FIXME: some attributes don't like getting set to undefined...
              elem[attribute] = undefined;
            }
            break;
        }
      }
    }
    // clean up any styles that need to go
    for(let styleId of stylesToGC) {
      activeStyles[styleId] = null;
    }
  }

  let styles = [];
  let entities = Object.keys(additions)
  for(let entId of entities) {
    let ent = additions[entId];
    let elem = activeElements[entId]
    // if we don't have an element already and this one doesn't
    // have a tag, then we just skip it (e.g. a style)
    if(!elem && !ent.tag)  continue;
    if(!elem) {
      //TODO: support finding the correct tag
      elem = document.createElement(ent.tag || "div")
      elem.entity = entId;
      activeElements[entId] = elem;
      elem.sort = ent.sort || "";
      insertSorted(activeElements.root, elem)
    }
    let attributes = Object.keys(ent);
    for(let attr of attributes) {
      let value = ent[attr];
      if(attr == "children") {
        for(let child of value) {
          let childElem = activeElements[child];
          if(childElem) {
            insertSorted(elem, childElem)
          } else {
            let childAddition = additions[child];
            // FIXME: if somehow you get a child id, but that child
            // has no facts provided, we'll just lose that information
            // here..
            if(childAddition) {
              childAddition._parent = entId;
            }
          }
        }
      } else if(attr == "style") {
        styles.push(value);
        activeStyles[value] = elem;
      } else if(attr == "textContent") {
        elem.textContent = value;
      } else if(attr == "tag" || attr == "ix") {
        //ignore
      } else if(attr == "_parent") {
        let parent = activeElements[value];
        insertSorted(parent, elem);
      } else {
        elem.setAttribute(attr, value);
      }
    }
  }

  for(let styleId of styles) {
    let style = additions[styleId];
    if(!style) continue;
    let elem = activeStyles[styleId];
    if(!elem) {
      console.error("Got a style for an element that doesn't exist.");
      continue;
    }
    let elemStyle = elem.style;
    let styleAttributes = Object.keys(style);
    for(let attr of styleAttributes) {
      elemStyle[attr] = style[attr];
    }
  }
}

// add our root to the body so that we update appropriately
document.body.appendChild(activeElements["root"])

//---------------------------------------------------------
// Helpers to send event update queries
//---------------------------------------------------------

function formatObjects(objs) {
  let rows = [];
  for(let obj of objs) {
    let fields = []
    for(let key in obj) {
      let value = obj[key];
      if(key == "tags") {
        for(let tag of value) {
          fields.push("#" + tag)
        }
      } else {
        let stringValue;
        if(typeof value == "string" && value[0] == "⦑") {
          stringValue = value
        } else {
          stringValue = JSON.stringify(value);
        }
        fields.push(key + ": " + stringValue);
      }
    }
    rows.push("[" + fields.join(", ") + "]")
  }
  return rows;
}

function sendEvent(objs) {
  let query = `handle some event
  update
    ${formatObjects(objs).join("\n    ")}
  `
  console.log("QUERY", query);
  if(socket && socket.readyState == 1) {
    socket.send(JSON.stringify({scope: "event", type: "query", query}))
  }
  return query;
}

//---------------------------------------------------------
// Event bindings to forward events to the server
//---------------------------------------------------------

window.addEventListener("click", function(event) {
  let {target} = event;
  let current = target;
  let objs = [];
  while(current) {
    if(current.entity) {
      objs.push({tags: ["click"], element: current.entity});
    }
    current = current.parentNode
  }
  objs.push({tags: ["click"], element: "window"});
  sendEvent(objs);
});

window.addEventListener("input", function(event) {
  let {target} = event;
  let objs = [{tags: ["input"], element: target.entity, value: target.value}];
  sendEvent(objs);
});

window.addEventListener("focus", function(event) {
  let {target} = event;
  console.log("FOCUS", event);
  if(target.entity) {
    let objs = [{tags: ["focus"], element: target.entity}];
    console.log(sendEvent(objs));
  }
}, true);

window.addEventListener("blur", function(event) {
  let {target} = event;
  if(target.entity) {
    let objs = [{tags: ["blur"], element: target.entity}];
    console.log(sendEvent(objs));
  }
}, true);

window.addEventListener("keydown", function(event) {
  let {target} = event;
  let current = target;
  let objs = [];
  let key = event.keyCode;
  while(current) {
    if(current.entity) {
      objs.push({tags: ["keydown"], element: current.entity, key});
    }
    current = current.parentNode
  }
  objs.push({tags: ["keydown"], element: "window", key});
  // sendEvent(objs);
});

window.addEventListener("keyup", function(event) {
  let {target} = event;
  let current = target;
  let objs = [];
  let key = event.keyCode;
  while(current) {
    if(current.entity) {
      objs.push({tags: ["keyup"], element: current.entity, key});
    }
    current = current.parentNode
  }
  objs.push({tags: ["keyup"], element: "window", key});
  // sendEvent(objs);
});

//---------------------------------------------------------
// Draw node graph
//---------------------------------------------------------

let allNodeGraphs = {};

let styles  = {
  root: "display:flex; flex-direction:column; justify-content:flex-start; align-items:flex-start; margin-top:20px;",
  graph: "margin-top:30px;",
  node: "display: flex; flex-direction:column; margin:0 0px;",
  nodeChildren: "display: flex; flex-direction:column; align-items:stretch; ",
  nodeType: "display:flex; justify-content:center; background: #ddd; margin: 0px; padding: 5px 10px;",
  subNodeChildren: "flex-direction:column; margin-left: 0px;",
  forkNodeChildren: "flex-direction:row; justify-content: center;",
}

function drawNode(nodeId, graph, seen) {
  let node = graph[nodeId];
  if(seen[nodeId]) {
    return {text: `seen ${node.type}`};
  } else if(node.type == "terminal" || node.type == "subtail") {
    return undefined;
  }
  seen[nodeId] = true;
  let children = [];
  let childrenContainer = {c: "node-children", children};
  let me = {c: "node", children: [
    {c: `${node.type} node-text`, text: `${node.type} ${node.scan_type || ""} (${node.count || 0})`},
    childrenContainer
  ]};
  if((node.type == "fork") || (node.type == "choose")) {
    childrenContainer.c += ` fork-node-children`;
    for(let child of node.arms) {
      children.push({style: "margin-right: 20px;", children: [drawNode(child, graph, seen)]});
    }
  } else if(node.type == "sub") {
    childrenContainer.c += ` sub-node-children`;
    children.push({style: "margin-left: 30px;", children: [drawNode(node.arms[1], graph, seen)]});
    children.push(drawNode(node.arms[0], graph, seen));
  } else {
    for(let child of node.arms) {
      children.push(drawNode(child, graph, seen));
    }
  }
  return me;
}

function drawNodeGraph(graph) {
  allNodeGraphs[graph.head] = graph;
  let graphs = [];
  for(let headId in allNodeGraphs) {
    let tree = drawNode(headId, allNodeGraphs[headId].nodes, {});
    graphs.push({c: "graph", children: [
      {text: `total time: ${allNodeGraphs[headId].total_time}s`},
      {text: `iterations: ${allNodeGraphs[headId].iterations}`},
      tree
    ]});
  }
  renderer.render([{c: "graph-root", children: graphs}]);
}

//---------------------------------------------------------
// Connect the websocket, send the ui code
//---------------------------------------------------------

var socket = new WebSocket("ws://" + window.location.host +"/ws");
socket.onmessage = function(msg) {
  console.log(msg.data)
  let data = JSON.parse(msg.data);
  if(data.type == "result") {
    handleDOMUpdates(data);
  } else if(data.type == "node_graph") {
    drawNodeGraph(data);
  }
}
socket.onopen = function() {
  console.log("Connected to eve server!");
}
