import { defineHastPlugin } from "satteri";

export default defineHastPlugin({
  name: "recite-markdown-keyboard-access",
  element: {
    filter: ["table", "pre"],
    visit(node, context) {
      context.setProperty(node, "tabIndex", 0);
    },
  },
});
