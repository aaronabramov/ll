import React, { useState } from "react";

import ReactDOM from "react-dom";
import { Graph, ParsedGraph } from "./Graph";
import type { Node } from "./Graph";

export function TreeTable() {
  const graph = new ParsedGraph().finalize();

  const [expanded, setExpanded] = useState({ ids: new Set() });

  const onClick = (id: number) => {
    let ids = expanded.ids;
    if (ids.has(id)) {
      ids.delete(id);
    } else {
      ids.add(id);
    }

    setExpanded({ ids });
  };

  const rows = [];
  const stack = graph.roots.map((r) => [r, 0]);

  while (stack.length !== 0) {
    const [next_id, indent] = stack.pop();

    const node = graph.get(next_id);
    const indent_str = new Array(indent).fill("• ").join("");

    rows.push(
      <Row
        node={node}
        graph={graph}
        indent={indent}
        onClick={onClick}
        expanded={expanded.ids.has(next_id)}
      />
    );

    if (expanded.ids.has(next_id)) {
      for (const child of node.data.children) {
        stack.push([child, indent + 1]);
      }
    }
  }

  return <div className="ll-table">{rows}</div>;
}

const Row = (props: {
  graph: Graph;
  node: Node;
  indent: number;
  onClick: (id: number) => void;
  expanded: Boolean;
}) => {
  const segments = props.node.data.name.split(":");
  const name = segments[segments.length - 1];

  const node = props.node;

  const earliest = props.graph.earliest;
  const latest = props.graph.latest;

  const start = node.data.start || earliest;
  const end = node.data.end || latest;

  const total = latest - earliest;

  const pre = ((start - earliest) / total) * 100;
  let span = ((end - start) / total) * 100;
  const post = ((latest - end) / total) * 100;

  if (span < 0.1) {
    span = 2;
  }

  const expanded = props.expanded ? "▼" : "►";

  return (
    <div
      className="ll-table-row"
      key={props.node.data.id}
      onClick={() => props.onClick(node.data.id)}
    >
      <div className="ll-row-name">
        <span style={{ width: `${props.indent * 10}px` }} />
        <span className="ll-row-expanded">{expanded}</span>
        {name}
      </div>
      <div className="ll-row-span">
        <span className="ll-row-span-pre" style={{ width: `${pre}px` }}></span>
        <span className="ll-row-span-span" style={{ width: `${span}%` }}></span>
        <span className="ll-row-span-post" style={{ width: `${post}%` }}></span>
      </div>
    </div>
  );
};
