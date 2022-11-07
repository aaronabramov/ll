const sampleData = `
{"name":"root","id":0,"event_type":"Start","unix_ts_millis":1667773318070}
{"name":"root:will_finish_fast","id":1,"event_type":"Start","parent_id":0,"unix_ts_millis":1667773318070}
{"name":"root:will_finish_fast","id":1,"event_type":"End","parent_id":0,"unix_ts_millis":1667773318070}
{"name":"root2","id":2,"event_type":"Start","unix_ts_millis":1667773318070}
{"name":"root:task_1","id":3,"event_type":"Start","parent_id":0,"unix_ts_millis":1667773318070}
{"name":"root2","id":2,"event_type":"End","unix_ts_millis":1667773318070}
{"name":"root:task_1:task_2","id":4,"event_type":"Start","parent_id":3,"unix_ts_millis":1667773319077}
{"name":"root:task_1:detached_async_task","id":5,"event_type":"Start","parent_id":3,"unix_ts_millis":1667773319077}
{"name":"root:task_1:task_2:task_2.5","id":6,"event_type":"Start","parent_id":4,"unix_ts_millis":1667773319128}
{"name":"root:task_1:task_2:won't be printed","id":7,"event_type":"Start","parent_id":4,"unix_ts_millis":1667773319128}
{"name":"root:task_1:task_3","id":8,"event_type":"Start","parent_id":3,"unix_ts_millis":1667773319128}
{"name":"root:task_1:task_2:task_2.5","id":6,"event_type":"End","parent_id":4,"unix_ts_millis":1667773319128}
{"name":"root:task_1:task_2:won't be printed","id":7,"event_type":"End","parent_id":4,"unix_ts_millis":1667773319128}
{"name":"root:task_1:task_3:task_4","id":9,"event_type":"Start","data":{"transitive":"555"},"parent_id":8,"unix_ts_millis":1667773321880}
{"name":"root:task_1:task_3:task_4:will_error","id":10,"event_type":"Start","data":{"transitive":"555"},"parent_id":9,"unix_ts_millis":1667773321927}
{"name":"root:task_1:task_3:task_4:will_error:hello","id":11,"event_type":"Start","data":{"transitive":"555"},"parent_id":10,"unix_ts_millis":1667773321927}
{"name":"root:task_1:task_3:task_4:will_error:hello","id":11,"event_type":"End","data":{"transitive":"555"},"parent_id":10,"unix_ts_millis":1667773321927}
{"name":"root:task_1:task_3:task_4:will_error","id":10,"event_type":"End","data":{"transitive":"555"},"parent_id":9,"unix_ts_millis":1667773321927}
{"name":"root:task_1:task_3:task_4:will_error:will run longer that parent","id":12,"event_type":"Start","data":{"transitive":"555"},"parent_id":10,"unix_ts_millis":1667773321927}
{"name":"will_spawn_after_parent_is_done","id":13,"event_type":"Start","parent_id":1,"unix_ts_millis":1667773324072}
{"name":"root:task_1:task_3:task_4","id":9,"event_type":"End","data":{"transitive":"555"},"parent_id":8,"unix_ts_millis":1667773325129}
{"name":"root:task_1:task_3","id":8,"event_type":"End","data":{"transitive":"555"},"parent_id":3,"unix_ts_millis":1667773327933}
{"name":"root:task_1:task_2","id":4,"event_type":"End","data":{"dontprint":"4","hey":"1","yo":"sup"},"parent_id":3,"unix_ts_millis":1667773330129}
{"name":"root:task_1:detached_async_task","id":5,"event_type":"End","parent_id":3,"unix_ts_millis":1667773330137}
{"name":"root:task_1","id":3,"event_type":"End","parent_id":0,"unix_ts_millis":1667773331131}
{"name":"root","id":0,"event_type":"End","unix_ts_millis":1667773331142}`;

enum EventType {
  Start,
  End,
}

type Event = {
  name: string;
  id: number;
  event_type: EventType;
  data: { string: string };
  parent_id?: number;
  unix_ts_millis?: number;
};

export class Graph {
  roots: number[];
  nodes: Map<number, Node> = new Map();

  earliest: number;
  latest: number;

  constructor(nodes: Map<number, NodeData>) {
    this.nodes = new Map();
    this.roots = [];
    nodes.forEach((node) => {
      this.nodes.set(node.id, new Node(node));

      if (node.start != null) {
        this.earliest =
          this.earliest == null
            ? node.start
            : Math.min(node.start, this.earliest || 0);
      }
      if (node.end != null) {
        this.latest =
          this.latest == null
            ? node.end
            : Math.max(node.end, this.earliest || 0);
      }

      if (node.parent_id == null) {
        this.roots.push(node.id);
      }
    });
  }

  get(id: number): Node {
    return this.nodes.get(id);
  }
}

const parseJSONL = (jsonl: string) => {
  const lines = jsonl.trim().split("\n");

  const graph = {};

  for (const line of lines) {
    const event = JSON.parse(line);
  }
};

export type NodeData = {
  name: string;
  children: number[];
  data: { [key: string]: string };
  id: number;
  parent_id?: number;
  start?: number;
  end?: number;
};

export class Node {
  data: NodeData;

  constructor(data: NodeData) {
    this.data = data;
  }

  id(): number {
    return this.data.id;
  }
}

export class ParsedGraph {
  nodes: Map<number, NodeData> = new Map();
  childToParent: Map<number, number> = new Map();

  constructor() {
    const jsonl = sampleData.trim();
    const lines = jsonl.split("\n");
    for (const line of lines) {
      const event: Event = JSON.parse(line);
      this.addEvent(event);
    }
  }

  addEvent(event: Event) {
    if (this.nodes.get(event.id) == null) {
      this.nodes.set(event.id, {
        name: event.name,
        children: [],
        data: {},
        parent_id: event.parent_id,
        id: event.id,
      });
    }

    const node = this.nodes.get(event.id);
    node.data = { ...(node.data || {}), ...event.data };

    if (node.parent_id != null) {
      this.childToParent.set(node.id, node.parent_id);
    }

    switch (event.event_type.toString()) {
      case "Start": {
        node.start = event.unix_ts_millis;
      }
      case "End": {
        node.end = event.unix_ts_millis;
      }
    }
  }

  finalize(): Graph {
    this.childToParent.forEach((parent, child) => {
      const node = this.nodes.get(parent);
      node.children.push(child);
    });

    return new Graph(this.nodes);
  }
}
