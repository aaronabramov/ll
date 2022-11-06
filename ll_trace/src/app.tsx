
import React from "react";
import ReactDOM from "react-dom";

import { TreeTable } from "./TreeTable";


export function App() {
    return <TreeTable />;
}

const app = document.getElementById("root");
ReactDOM.render(<App />, app);



