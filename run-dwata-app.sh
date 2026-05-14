#!/usr/bin/env bash
set -e

echo "Installing GUI dependencies..."
npm --prefix gui install

echo "Installing Tauri CLI dependencies..."
npm --prefix tauri install

echo "Building dwata-api (Tauri sidecar)..."
cargo build -p dwata-api --bin dwata-api

echo "Starting Dwata Tauri app..."
echo ""

npm --prefix tauri run dev
