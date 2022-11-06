
import React, { useState } from "react";

import ReactDOM from "react-dom";

export function TreeTable() {
    let entries = new Array(50).fill(null).map((e, i) => `meowdy-${i}`);
    return <div className="ll-table"><List entries={entries} /></div>;
}

export function List(props: { entries: string[] }) {
    const rows = props.entries.map((e, i) => <div className="ll-table-row" key={i}>{e}</div>);
    return <React.Fragment>{rows}</React.Fragment>;
}