import { defineScript, t } from "@meowlink/script";

export default defineScript({
  input: {
    samples: t.list(t.number),
    delay: t.duration,
    image: t.image,
    settings: t.object,
  },
  output: {
    average: t.number,
    delay: t.duration,
    image: t.image,
    settings: t.object,
  },
  async run({ samples, delay, image, settings }) {
    return {
      average: samples.reduce((sum, value) => sum + value, 0) / samples.length,
      delay,
      image,
      settings,
    };
  },
});
