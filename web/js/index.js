import * as wasm from "initiative-web"
import { export_database, import_database } from "./database"
import terminal from "./terminal"

const terminalElement = document.getElementById("terminal")

terminal.initialize(
  "terminal",
  async (query) => (await wasm.autocomplete(query)).map(a => {
    return {
      suggestion: a[0],
      description: a[1],
    }
  }),
)

wasm.initialize("terminal")
  .then((motd) => terminal.output(motd))
  .catch((err) => console.log(err))

terminalElement.addEventListener(
  "initiative.export",
  async (event) => await export_database(event.detail),
)

terminalElement.addEventListener(
  "initiative.startImport",
  async (event) => await import_database(),
)

terminalElement.addEventListener(
  "initiative.command",
  async (event) => terminal.output(await wasm.command(event.detail.command)),
)
