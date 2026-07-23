import test from "node:test";
import assert from "node:assert/strict";
import {
  filterPatchHiveTextModels,
  isAgentReadyProviderModel,
} from "./modelFilters.js";

const agentMetadata = {
  architecture: {
    input_modalities: ["text"],
    output_modalities: ["text"],
  },
  context_length: 131_072,
  supported_parameters: ["max_tokens", "tools", "structured_outputs"],
};

test("agent-ready metadata keeps capable text models", () => {
  assert.equal(isAgentReadyProviderModel("vendor/coder", agentMetadata), true);
});

test("agent-ready metadata rejects short-context and uncontrolled models", () => {
  assert.equal(isAgentReadyProviderModel("vendor/short", {
    ...agentMetadata,
    context_length: 8_192,
  }), false);
  assert.equal(isAgentReadyProviderModel("vendor/chat", {
    ...agentMetadata,
    supported_parameters: ["max_tokens", "temperature"],
  }), false);
});

test("agent-ready and free filters remain independent", () => {
  const metadata = {
    "vendor/agent:free": agentMetadata,
    "vendor/agent-paid": agentMetadata,
    "vendor/basic:free": {
      ...agentMetadata,
      supported_parameters: ["max_tokens"],
    },
  };

  const agentReady = filterPatchHiveTextModels(Object.keys(metadata), {
    agentReadyOnly: true,
    metadata,
  });
  assert.deepEqual(agentReady.models, ["vendor/agent:free", "vendor/agent-paid"]);
  assert.deepEqual(agentReady.agentHidden, ["vendor/basic:free"]);

  const agentReadyAndFree = filterPatchHiveTextModels(Object.keys(metadata), {
    agentReadyOnly: true,
    freeOnly: true,
    metadata,
  });
  assert.deepEqual(agentReadyAndFree.models, ["vendor/agent:free"]);
  assert.deepEqual(agentReadyAndFree.freeHidden, ["vendor/agent-paid"]);
});
