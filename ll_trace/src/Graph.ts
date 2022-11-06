export type NodeID = string;
export type Node = {
    children: Array<NodeID>,
}

export class Graph {
    roots: Array<NodeID>;
    nodes: { NodeID: Node };

    constructor() {
        this.roots[nextID()]
    }

    getNode(id: NodeID) {
        let node = this.nodes[id];

        if (node != null) {
            return node;
        } else {
            node = { children: [nextID(), nextID(), nextID()] };
            this.nodes[id] = node;
            return node;
        }
    }
}


let id = 1;
const nextID = () => (id++).toString();