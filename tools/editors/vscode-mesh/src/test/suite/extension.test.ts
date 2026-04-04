import assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

interface MeshExtensionApi {
  resolvedMeshcPath: string;
  resolutionSource: string;
}

interface OverrideEntryFixture {
  projectDir: string;
  manifestPath: string;
  entryPath: string;
  supportPath: string;
  entrySource: string;
}

function requiredEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`[setup/env] Missing required ${name} environment variable.`);
  }
  return value;
}

const artifactDir = requiredEnv("MESH_VSCODE_SMOKE_ARTIFACT_DIR");
const logFile = requiredEnv("MESH_VSCODE_SMOKE_LOG_FILE");
const repoRoot = requiredEnv("MESH_VSCODE_SMOKE_REPO_ROOT");
const meshcPath = requiredEnv("MESH_VSCODE_SMOKE_MESHC_PATH");
const workspaceFile = requiredEnv("MESH_VSCODE_SMOKE_WORKSPACE_FILE");
const extensionId = "OpenWorthTechnologies.mesh-lang";
const retainedReferenceBackendRoot = path.join(
  repoRoot,
  "scripts",
  "fixtures",
  "backend",
  "reference-backend"
);
const retainedHealthPath = path.join(
  retainedReferenceBackendRoot,
  "api",
  "health.mpl"
);
const retainedJobsPath = path.join(
  retainedReferenceBackendRoot,
  "api",
  "jobs.mpl"
);

function log(message: string) {
  const line = `[smoke] ${message}`;
  fs.mkdirSync(artifactDir, { recursive: true });
  fs.appendFileSync(logFile, `${line}\n`);
  console.error(line);
}

async function withTimeout<T>(
  label: string,
  promise: Thenable<T> | Promise<T>,
  timeoutMs = 15000
): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  try {
    return await Promise.race([
      Promise.resolve(promise),
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => {
          reject(new Error(`[${label}] Timed out after ${timeoutMs}ms.`));
        }, timeoutMs);
      }),
    ]);
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
}

async function waitForCondition(
  label: string,
  predicate: () => boolean,
  timeoutMs = 10000,
  intervalMs = 50
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  throw new Error(`[${label}] Timed out after ${timeoutMs}ms.`);
}

class DiagnosticsTracker {
  private seen = new Set<string>();
  private waiters = new Map<string, Array<(diagnostics: readonly vscode.Diagnostic[]) => void>>();
  private readonly disposable: vscode.Disposable;

  constructor() {
    this.disposable = vscode.languages.onDidChangeDiagnostics((event) => {
      for (const uri of event.uris) {
        const key = uri.toString();
        const diagnostics = vscode.languages.getDiagnostics(uri);
        this.seen.add(key);
        const waiters = this.waiters.get(key);
        if (!waiters) {
          continue;
        }
        this.waiters.delete(key);
        for (const resolve of waiters) {
          resolve(diagnostics);
        }
      }
    });
  }

  async waitFor(uri: vscode.Uri, label: string, timeoutMs = 15000) {
    const key = uri.toString();
    if (this.seen.has(key)) {
      return vscode.languages.getDiagnostics(uri);
    }

    return await withTimeout(
      label,
      new Promise<readonly vscode.Diagnostic[]>((resolve) => {
        const waiters = this.waiters.get(key) ?? [];
        waiters.push(resolve);
        this.waiters.set(key, waiters);
      }),
      timeoutMs
    );
  }

  dispose() {
    this.disposable.dispose();
    this.waiters.clear();
  }
}

function sourcePosition(source: string, needle: string, occurrence = 0): vscode.Position {
  const matches = [...source.matchAll(new RegExp(needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g"))];
  const match = matches[occurrence];
  if (!match || typeof match.index !== "number") {
    throw new Error(
      `[probe/source-position] Could not find occurrence ${occurrence} of ${JSON.stringify(
        needle
      )}.`
    );
  }

  const prefix = source.slice(0, match.index);
  const line = prefix.split("\n").length - 1;
  const lastNewline = prefix.lastIndexOf("\n");
  const character = prefix.slice(lastNewline + 1).length;
  return new vscode.Position(line, character);
}

function hoverText(hovers: readonly vscode.Hover[] | undefined): string {
  return (hovers ?? [])
    .flatMap((hover) => hover.contents)
    .map((content) => {
      if (typeof content === "string") {
        return content;
      }
      if (content instanceof vscode.MarkdownString) {
        return content.value;
      }
      return content.value;
    })
    .join("\n");
}

function definitionTargets(
  definitions: readonly (vscode.Location | vscode.LocationLink)[] | undefined
): Array<{ uri: vscode.Uri; startLine: number }> {
  return (definitions ?? []).map((entry) => {
    if (entry instanceof vscode.Location) {
      return {
        uri: entry.uri,
        startLine: entry.range.start.line,
      };
    }

    return {
      uri: entry.targetUri,
      startLine: entry.targetRange.start.line,
    };
  });
}

async function openDocument(filePath: string, label: string) {
  const uri = vscode.Uri.file(filePath);
  const document = await withTimeout(
    `${label}/open`,
    vscode.workspace.openTextDocument(uri)
  );
  await withTimeout(`${label}/show`, vscode.window.showTextDocument(document, { preview: false }));
  await waitForCondition(
    `${label}/languageId`,
    () => document.languageId === "mesh",
    5000
  );
  log(`${label} opened as languageId=${document.languageId}: ${filePath}`);
  return document;
}

function resetDir(dirPath: string) {
  fs.rmSync(dirPath, { recursive: true, force: true });
  fs.mkdirSync(dirPath, { recursive: true });
}

function writeFixtureFile(filePath: string, contents: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, contents, "utf8");
}

function materializeOverrideEntryFixture(): OverrideEntryFixture {
  const workspaceDir = path.dirname(workspaceFile);
  const projectDir = path.join(workspaceDir, "override-entry-project");
  const manifestPath = path.join(projectDir, "mesh.toml");
  const entryPath = path.join(projectDir, "lib", "start.mpl");
  const supportPath = path.join(projectDir, "lib", "support", "message.mpl");
  const entrySource = [
    "from Lib.Support.Message import message",
    "",
    "fn main() do",
    "  let rendered = message()",
    '  println("proof=#{rendered}")',
    "end",
    "",
  ].join("\n");
  const supportSource = [
    "pub fn message() -> String do",
    '  "nested-support"',
    "end",
    "",
  ].join("\n");

  resetDir(projectDir);
  writeFixtureFile(
    manifestPath,
    ['[package]', 'name = "override-entry-project"', 'version = "0.1.0"', 'entrypoint = "lib/start.mpl"', ''].join("\n")
  );
  writeFixtureFile(entryPath, entrySource);
  writeFixtureFile(supportPath, supportSource);

  log(
    `Materialized override-entry fixture project=${projectDir} manifest=${manifestPath} entry=${entryPath} support=${supportPath}`
  );

  return {
    projectDir,
    manifestPath,
    entryPath,
    supportPath,
    entrySource,
  };
}

async function assertCleanDiagnostics(
  diagnostics: DiagnosticsTracker,
  document: vscode.TextDocument,
  label: string
) {
  log(`Waiting for clean diagnostics on ${document.uri.fsPath}`);
  const entries = await diagnostics.waitFor(document.uri, label);
  assert.equal(
    entries.length,
    0,
    `[${label}] Expected no diagnostics for ${document.uri.fsPath}, got ${entries
      .map((diagnostic) => diagnostic.message)
      .join(" | ")}.`
  );
}

export async function runSmokeSuite(): Promise<void> {
  const diagnostics = new DiagnosticsTracker();

  try {
    const currentWorkspace = vscode.workspace.workspaceFile?.fsPath;
    assert.equal(
      currentWorkspace,
      workspaceFile,
      `[setup/workspace] Expected workspace file ${workspaceFile}, got ${currentWorkspace ?? "<none>"}.`
    );

    const configuredPath = vscode.workspace
      .getConfiguration("mesh.lsp")
      .get<string>("path");
    assert.equal(
      configuredPath,
      meshcPath,
      `[setup/config] Expected mesh.lsp.path=${meshcPath}, got ${configuredPath ?? "<missing>"}.`
    );
    log(`Configured mesh.lsp.path=${configuredPath}`);

    const extension = vscode.extensions.getExtension<MeshExtensionApi>(extensionId);
    assert.ok(extension, `[activation] Missing extension ${extensionId}.`);

    log(`Using retained backend fixture root ${retainedReferenceBackendRoot}`);
    const healthDocument = await openDocument(retainedHealthPath, "health");

    log(`Waiting for extension activation via ${extensionId}`);
    const api = await withTimeout(
      "activation",
      extension!.activate() as Thenable<MeshExtensionApi>,
      15000
    );
    assert.equal(
      api.resolvedMeshcPath,
      meshcPath,
      `[activation] Expected resolved meshc path ${meshcPath}, got ${api.resolvedMeshcPath}.`
    );
    assert.equal(
      api.resolutionSource,
      "configuration",
      `[activation] Expected resolution source configuration, got ${api.resolutionSource}.`
    );
    log(`Extension resolved meshc from ${api.resolutionSource}: ${api.resolvedMeshcPath}`);

    await assertCleanDiagnostics(diagnostics, healthDocument, "diagnostics/health");

    const jobsDocument = await openDocument(retainedJobsPath, "jobs");
    await assertCleanDiagnostics(diagnostics, jobsDocument, "diagnostics/jobs");

    const overrideFixture = materializeOverrideEntryFixture();
    const overrideEntryDocument = await openDocument(
      overrideFixture.entryPath,
      "override-entry-entry"
    );
    await assertCleanDiagnostics(
      diagnostics,
      overrideEntryDocument,
      "diagnostics/override-entry-entry"
    );

    const overrideSupportDocument = await openDocument(
      overrideFixture.supportPath,
      "override-entry-support"
    );
    await assertCleanDiagnostics(
      diagnostics,
      overrideSupportDocument,
      "diagnostics/override-entry-support"
    );

    const jobsSource = fs.readFileSync(retainedJobsPath, "utf8");
    const createJobCallMarker = "create_job_response(job, body)";
    const createJobDefinitionMarker =
      "fn create_job_response(job :: Job, payload :: String) do";
    const createJobCallPosition = sourcePosition(jobsSource, createJobCallMarker, 0);
    const createJobDefinitionPosition = sourcePosition(
      jobsSource,
      createJobDefinitionMarker,
      0
    );

    log(
      `Probing hover at ${jobsDocument.uri.fsPath}:${createJobCallPosition.line}:${createJobCallPosition.character}`
    );
    const hovers = await withTimeout(
      "probe/hover",
      vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        jobsDocument.uri,
        createJobCallPosition
      )
    );
    const hoverSummary = hoverText(hovers).trim();
    assert.ok(
      hoverSummary.length > 0,
      `[probe/hover] Hover returned no content for ${jobsDocument.uri.fsPath}.`
    );
    assert.ok(
      hoverSummary.includes("create_job_response") ||
        hoverSummary.includes("Job") ||
        hoverSummary.includes("String"),
      `[probe/hover] Hover content drifted for ${jobsDocument.uri.fsPath}: ${hoverSummary}`
    );
    log(`Hover probe returned ${JSON.stringify(hoverSummary)}`);

    log(
      `Probing definition at ${jobsDocument.uri.fsPath}:${createJobCallPosition.line}:${createJobCallPosition.character}`
    );
    const definitions = await withTimeout(
      "probe/definition",
      vscode.commands.executeCommand<(vscode.Location | vscode.LocationLink)[]>(
        "vscode.executeDefinitionProvider",
        jobsDocument.uri,
        createJobCallPosition
      )
    );
    const targets = definitionTargets(definitions);
    assert.ok(
      targets.length > 0,
      `[probe/definition] Definition returned no target for ${jobsDocument.uri.fsPath}.`
    );
    assert.ok(
      targets.some(
        (target) =>
          target.uri.fsPath === retainedJobsPath &&
          target.startLine === createJobDefinitionPosition.line
      ),
      `[probe/definition] Expected definition target ${retainedJobsPath}:${createJobDefinitionPosition.line}, got ${JSON.stringify(
        targets.map((target) => ({
          file: target.uri.fsPath,
          line: target.startLine,
        }))
      )}.`
    );
    log(
      `Definition probe resolved to ${retainedJobsPath}:${createJobDefinitionPosition.line}`
    );

    const overrideMessageCallPosition = sourcePosition(
      overrideFixture.entrySource,
      "message()",
      0
    );
    log(
      `Probing override-entry hover at ${overrideEntryDocument.uri.fsPath}:${overrideMessageCallPosition.line}:${overrideMessageCallPosition.character}`
    );
    const overrideHovers = await withTimeout(
      "probe/override-hover",
      vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        overrideEntryDocument.uri,
        overrideMessageCallPosition
      )
    );
    const overrideHoverSummary = hoverText(overrideHovers).trim();
    assert.ok(
      overrideHoverSummary.length > 0,
      `[probe/override-hover] Hover returned no content for ${overrideEntryDocument.uri.fsPath}.`
    );
    assert.ok(
      overrideHoverSummary.includes("String"),
      `[probe/override-hover] Expected imported nested helper type information for ${overrideEntryDocument.uri.fsPath}, got ${overrideHoverSummary}`
    );
    log(`Override-entry hover probe returned ${JSON.stringify(overrideHoverSummary)}`);
  } finally {
    diagnostics.dispose();
  }
}
