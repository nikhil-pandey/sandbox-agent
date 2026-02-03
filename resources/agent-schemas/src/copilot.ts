import { execSync } from "child_process";
import { existsSync, readFileSync, readdirSync } from "fs";
import { join } from "path";
import { createNormalizedSchema, type NormalizedSchema } from "./normalize.js";
import type { JSONSchema7 } from "json-schema";

/**
 * Find the latest Copilot CLI package directory containing schemas.
 * Looks in ~/.copilot/pkg/universal/ for versioned directories.
 */
function findCopilotSchemaDir(): string | null {
  const homeDir = process.env.HOME || process.env.USERPROFILE || "";
  const pkgDir = join(homeDir, ".copilot", "pkg", "universal");

  if (!existsSync(pkgDir)) {
    return null;
  }

  // Get all version directories and sort to find latest
  const versions = readdirSync(pkgDir)
    .filter((name) => /^\d+\.\d+\.\d+/.test(name))
    .sort((a, b) => {
      // Parse version numbers for proper comparison
      const parseVersion = (v: string) => {
        const match = v.match(/^(\d+)\.(\d+)\.(\d+)/);
        if (!match) return [0, 0, 0];
        return [parseInt(match[1]), parseInt(match[2]), parseInt(match[3])];
      };
      const [aMajor, aMinor, aPatch] = parseVersion(a);
      const [bMajor, bMinor, bPatch] = parseVersion(b);
      if (aMajor !== bMajor) return bMajor - aMajor;
      if (aMinor !== bMinor) return bMinor - aMinor;
      return bPatch - aPatch;
    });

  if (versions.length === 0) {
    return null;
  }

  // Return the schemas directory of the latest version
  const latestVersion = versions[0];
  const schemasDir = join(pkgDir, latestVersion, "schemas");

  if (existsSync(schemasDir)) {
    return schemasDir;
  }

  return null;
}

export async function extractCopilotSchema(): Promise<NormalizedSchema> {
  console.log("Extracting Copilot CLI schema...");

  try {
    // First, try to find the schema file from the installed CLI
    const schemasDir = findCopilotSchemaDir();

    if (schemasDir) {
      const schemaPath = join(schemasDir, "session-events.schema.json");

      if (existsSync(schemaPath)) {
        console.log(`  [ok] Found schema at ${schemaPath}`);
        const content = readFileSync(schemaPath, "utf-8");
        const schema = JSON.parse(content);

        // The schema is a single SessionEvent type with anyOf for all event types
        const definitions: Record<string, JSONSchema7> = {};

        if (schema.definitions) {
          for (const [name, def] of Object.entries(schema.definitions)) {
            definitions[name] = def as JSONSchema7;
          }
        } else {
          // Wrap the root schema as SessionEvent
          definitions["SessionEvent"] = schema as JSONSchema7;
        }

        console.log(
          `  [ok] Extracted ${Object.keys(definitions).length} definitions`
        );
        return createNormalizedSchema(
          "copilot",
          "Copilot CLI Session Events Schema",
          definitions
        );
      }
    }

    // Try to get version info from CLI
    try {
      const versionOutput = execSync("copilot --version", {
        encoding: "utf-8",
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      });
      console.log(`  [info] Copilot CLI version: ${versionOutput.trim()}`);
    } catch {
      // Version check failed, continue with fallback
    }

    console.log("  [warn] Could not find installed schema, using fallback");
    return createFallbackSchema();
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.log(`  [warn] Schema extraction failed: ${errorMessage}`);
    console.log("  [fallback] Using embedded schema definitions");
    return createFallbackSchema();
  }
}

function createFallbackSchema(): NormalizedSchema {
  // Fallback schema based on observed Copilot CLI event structure
  const definitions: Record<string, JSONSchema7> = {
    SessionEvent: {
      type: "object",
      properties: {
        id: { type: "string", format: "uuid" },
        timestamp: { type: "string", format: "date-time" },
        parentId: {
          anyOf: [{ type: "string", format: "uuid" }, { type: "null" }],
        },
        ephemeral: { type: "boolean" },
        type: { type: "string" },
        data: { type: "object" },
      },
      required: ["id", "timestamp", "parentId", "type", "data"],
    },
    SessionStartData: {
      type: "object",
      properties: {
        sessionId: { type: "string" },
        version: { type: "number" },
        producer: { type: "string" },
        copilotVersion: { type: "string" },
        startTime: { type: "string", format: "date-time" },
        selectedModel: { type: "string" },
        context: {
          type: "object",
          properties: {
            cwd: { type: "string" },
            gitRoot: { type: "string" },
            repository: { type: "string" },
            branch: { type: "string" },
          },
          required: ["cwd"],
        },
      },
      required: ["sessionId", "version", "producer", "copilotVersion", "startTime"],
    },
    UserMessageData: {
      type: "object",
      properties: {
        content: { type: "string" },
        transformedContent: { type: "string" },
        attachments: {
          type: "array",
          items: {
            type: "object",
            properties: {
              type: { type: "string", enum: ["file", "directory"] },
              path: { type: "string" },
              displayName: { type: "string" },
            },
          },
        },
        source: { type: "string" },
      },
      required: ["content"],
    },
    AssistantMessageData: {
      type: "object",
      properties: {
        messageId: { type: "string" },
        content: { type: "string" },
        toolRequests: {
          type: "array",
          items: {
            type: "object",
            properties: {
              toolCallId: { type: "string" },
              name: { type: "string" },
              arguments: {},
              type: { type: "string", enum: ["function", "custom"] },
            },
            required: ["toolCallId", "name"],
          },
        },
        parentToolCallId: { type: "string" },
      },
      required: ["messageId", "content"],
    },
    ToolExecutionStartData: {
      type: "object",
      properties: {
        toolCallId: { type: "string" },
        toolName: { type: "string" },
        arguments: {},
        parentToolCallId: { type: "string" },
      },
      required: ["toolCallId", "toolName"],
    },
    ToolExecutionCompleteData: {
      type: "object",
      properties: {
        toolCallId: { type: "string" },
        success: { type: "boolean" },
        isUserRequested: { type: "boolean" },
        result: {
          type: "object",
          properties: {
            content: { type: "string" },
          },
        },
        error: {
          type: "object",
          properties: {
            message: { type: "string" },
            code: { type: "string" },
          },
        },
        toolTelemetry: { type: "object" },
        parentToolCallId: { type: "string" },
      },
      required: ["toolCallId", "success"],
    },
    SessionErrorData: {
      type: "object",
      properties: {
        errorType: { type: "string" },
        message: { type: "string" },
        stack: { type: "string" },
      },
      required: ["errorType", "message"],
    },
  };

  return createNormalizedSchema(
    "copilot",
    "Copilot CLI Session Events Schema (Fallback)",
    definitions
  );
}
