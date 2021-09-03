import * as wasm from "initiative-web";
import autoComplete from "@tarekraafat/autocomplete.js";
import marked from "marked";

document.body.insertAdjacentHTML(
  "beforeend",
  "<form id=\"prompt-form\"><input type=\"text\" id=\"prompt\" autocomplete=\"off\" autocorrect=\"off\" autocapitalize=\"none\"></form>"
);

const promptFormElement = document.getElementById("prompt-form");
const promptElement = document.getElementById("prompt");
const outputElement = document.getElementById("output");

const reducedMotion = (() => {
  const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
  return mediaQuery && mediaQuery.matches;
})();

marked.use({
  renderer: {
    del: (text) => `<code class="temp-link">${text}</code>`,
    link: (href, _, text) => `<a href="${href}" target="_blank">${text}</a>`,
  },
  extensions: [
    {
      name: "error",
      level: "block",
      start: (src) => src.match(/^! /)?.index,
      tokenizer: (src, tokens) => {
        const match = /^! (.+)$/.exec(src);
        if (match) {
          return { type: "error", raw: match[0], text: match[1].trim() };
        }
      },
      renderer: (token) => `<p class="error">${token.text}</p>`,
    },
  ],
});

const autoCompleteJS = new autoComplete({
  data: {
    src: async (query) => (await wasm.autocomplete(query)).map(a => {
      return {
        suggestion: a[0],
        description: a[1],
      };
    }),
    keys: ["suggestion"],
  },
  query: (input) => input.split("[")[0],
  resultsList: {
    class: "autocomplete-list",
  },
  resultItem: {
    class: "autocomplete-item",
    element: (item, data) => {
      item.innerHTML = `
      <span class="autocomplete-item-primary">${data.match}</span>
      <span class="autocomplete-item-description">${data.value.description}</span>
      `;
    },
    highlight: "autocomplete-item-highlight",
    selected: "autocomplete-item-selected",
  },
  selector: "#prompt",
  submit: true,
  wrapper: false,
});

const selectBracketedExpression = (command) => {
  promptElement.value = command;

  const match = /\[[^\]]+\]/.exec(command);
  if (!match) {
    return false;
  }

  promptElement.focus();
  promptElement.setSelectionRange(
    match.index,
    match.index + match[0].length,
  );

  if (!autoCompleteJS.isOpen) {
    autoCompleteJS.start();
  }

  return true;
};

const runCommand = async (command) => {
  if (!selectBracketedExpression(command)) {
    output("\\> " + command + "\n\n" + await wasm.command(command));
  }
};

const output = (text) => {
  outputElement.insertAdjacentHTML(
    "beforeend",
    marked(text)
  );

  promptElement.value = "";
  autoCompleteJS.close();
  window.scroll({
    left: 0,
    top: document.body.clientHeight,
    behavior: reducedMotion ? "auto" : "smooth",
  });
};

promptFormElement.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (promptElement.value !== "") {
    await runCommand(promptElement.value);
  }
});

promptFormElement.addEventListener("navigate", (event) => {
  selectBracketedExpression(event.detail.selection.value.suggestion);
});

promptFormElement.addEventListener("selection", async (event) => {
  if (event.detail.event.type == "click") {
    await runCommand(event.detail.selection.value.suggestion);
  }
});

window.addEventListener("keydown", (event) => {
  if (event.key === "Tab") {
    event.preventDefault();

    if (autoCompleteJS.isOpen &&
      (autoCompleteJS.feedback.results.length == 1 || autoCompleteJS.cursor > -1))
    {
      let index = Math.max(autoCompleteJS.cursor, 0);

      selectBracketedExpression(
        autoCompleteJS.feedback.results[index].value.suggestion
      );

      autoCompleteJS.start();
    }
  } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
    promptElement.focus();
  }
});

outputElement.addEventListener("click", async (event) => {
  if (event.target.nodeName === "CODE") {
    await runCommand(event.target.innerText);
  }
});

wasm.initialize()
  .then((motd) => output(motd))
  .catch((err) => console.log(err));

promptElement.focus();
