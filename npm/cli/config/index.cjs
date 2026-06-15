"use strict";

function defineConfig(config) {
  return config;
}

function tray(config) {
  return { type: "tray", ...config };
}

module.exports = { defineConfig, tray };
