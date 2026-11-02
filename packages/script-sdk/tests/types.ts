import { defineScript, t, type Value, type Temperature } from "../src/index.js";

const script = defineScript({
  input: {
    samples: t.list(t.number),
    matrix: t.list(t.list(t.number)),
    settings: t.object,
    dynamic: t.value,
    temperature: t.temperature,
  },
  output: { average: t.number, temperature: t.temperature },
  run({ samples, matrix, settings, dynamic, temperature }) {
    const sample: number | undefined = samples[0];
    const row: number[] | undefined = matrix[0];
    const mode: Value | undefined = settings["mode"];
    const anything: Value = dynamic;
    const original: Temperature = temperature;
    void [sample, row, mode, anything, original];
    return { average: samples.reduce((sum, value) => sum + value, 0), temperature };
  },
});

script({ samples: [1, 2], matrix: [[3]], settings: { nested: { x: 1 } }, dynamic: null,
  temperature: { _type: "Temperature", value: 20, unit: "C" } });

const domain: Temperature = { _type: "Temperature", value: 68, unit: "F" };
const dynamicDomain: Value = { nested: { temperature: domain } };
void dynamicDomain;

// @ts-expect-error named fields, including dynamic Value, are required
script({ samples: [1], matrix: [[2]], settings: {} });
// @ts-expect-error lists must retain element types and nesting
script({ samples: ["bad"], matrix: [1], settings: {}, dynamic: null, temperature: null });

defineScript({ input: { x: t.number }, output: { answer: t.number },
  // @ts-expect-error return type must follow output, not broaden its inference
  run: ({ x }) => ({ answer: String(x) }),
});
defineScript({ input: {}, output: { answer: t.number },
  // @ts-expect-error async output has the same contract
  run: async () => ({ answer: "bad" }),
});
defineScript({ input: {}, output: { items: t.list(t.number) },
  // @ts-expect-error root lists are represented by a named field
  run: () => [1, 2],
});
defineScript({ input: {}, output: { items: t.list(t.number) },
  run: async () => ({ items: [1, 2] }),
});

// @ts-expect-error this SDK does not provide ambient Node APIs
process.env.SECRET;
// @ts-expect-error browser DOM APIs are not part of the script environment
document.cookie;
