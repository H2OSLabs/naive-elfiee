# Part 6: Tauri App Interface Implementation

**Priority**: Critical | **Estimated Time**: 1.5 weeks

## Overview

Connect Rust backend to React frontend via Tauri v2 IPC. Build minimal UI with shadcn components for block editing. Integrate with EngineManager for multi-file support.

## Part 6A: Tauri Backend Commands

### Directory Structure

```
src-tauri/src/
├── commands/
│   ├── mod.rs
│   ├── file.rs      # File operations
│   └── block.rs     # Block operations
├── state.rs         # Tauri app state
└── main.rs
```

### Step 1: Define App State with EngineManager

**File**: `src-tauri/src/state.rs`

```rust
use crate::elf::ElfArchive;
use crate::engine::EngineManager;
use dashmap::DashMap;
use std::sync::Arc;

pub struct AppState {
    pub engine_manager: EngineManager,
    pub archives: Arc<DashMap<String, Arc<ElfArchive>>>, // file_id -> archive
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine_manager: EngineManager::new(),
            archives: Arc::new(DashMap::new()),
        }
    }
}
```

### Step 2: File Operations Commands

**File**: `src-tauri/src/commands/file.rs`

```rust
use crate::capabilities::CapabilityRegistry;
use crate::elf::ElfArchive;
use crate::engine::EventStore;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn open_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Generate file ID from path
    let file_id = format!("file-{}", uuid::Uuid::new_v4());

    // Open archive
    let archive = ElfArchive::open(&path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    // Load event store
    let event_store_path = archive.event_store_path();
    let event_store = EventStore::new(event_store_path.to_str().unwrap())
        .map_err(|e| format!("Failed to load event store: {}", e))?;

    // Create capability registry
    let registry = CapabilityRegistry::new();

    // Spawn engine actor
    let _handle = state
        .engine_manager
        .spawn_engine(file_id.clone(), event_store, registry)
        .await?;

    // Store archive
    state.archives.insert(file_id.clone(), Arc::new(archive));

    Ok(file_id)
}

#[tauri::command]
pub async fn create_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Generate file ID
    let file_id = format!("file-{}", uuid::Uuid::new_v4());

    // Create new archive
    let archive = ElfArchive::new()
        .map_err(|e| format!("Failed to create archive: {}", e))?;

    // Save immediately
    archive
        .save(&path)
        .map_err(|e| format!("Failed to save file: {}", e))?;

    // Load event store
    let event_store_path = archive.event_store_path();
    let event_store = EventStore::new(event_store_path.to_str().unwrap())
        .map_err(|e| format!("Failed to create event store: {}", e))?;

    // Create capability registry
    let registry = CapabilityRegistry::new();

    // Spawn engine actor
    let _handle = state
        .engine_manager
        .spawn_engine(file_id.clone(), event_store, registry)
        .await?;

    // Store archive
    state.archives.insert(file_id.clone(), Arc::new(archive));

    Ok(file_id)
}

#[tauri::command]
pub async fn save_file(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let archive = state
        .archives
        .get(&file_id)
        .ok_or("File not found")?;

    archive
        .save_to_original()
        .map_err(|e| format!("Failed to save: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn close_file(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Shutdown engine actor
    state.engine_manager.shutdown_engine(&file_id).await?;

    // Remove archive
    state.archives.remove(&file_id);

    Ok(())
}

#[tauri::command]
pub async fn list_open_files(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.engine_manager.list_files())
}
```

### Step 3: Block Operations Commands

**File**: `src-tauri/src/commands/block.rs`

```rust
use crate::models::{Block, Command};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn execute_command(
    file_id: String,
    cmd: Command,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Event>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("File not open")?;

    handle.process_command(cmd).await
}

#[tauri::command]
pub async fn get_block(
    file_id: String,
    block_id: String,
    state: State<'_, AppState>,
) -> Result<Block, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("File not open")?;

    handle
        .get_block(block_id)
        .await?
        .ok_or_else(|| format!("Block not found"))
}

#[tauri::command]
pub async fn get_all_blocks(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Block>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("File not open")?;

    handle.get_all_blocks().await
}
```

### Step 4: Wire Up Commands in main.rs

**File**: `src-tauri/src/main.rs`

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod engine;
mod elf;
mod capabilities;
mod commands;
mod state;

fn main() {
    tauri::Builder::default()
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::open_file,
            commands::create_file,
            commands::save_file,
            commands::close_file,
            commands::list_open_files,
            commands::execute_command,
            commands::get_block,
            commands::get_all_blocks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Step 5: Update lib.rs

**File**: `src-tauri/src/lib.rs`

```rust
pub mod models;
pub mod engine;
pub mod elf;
pub mod capabilities;
```

## Part 6B: React Frontend (Tauri v2 Compatible)

### Step 1: Install Dependencies

```bash
# Install Tauri dialog plugin (v2)
pnpm add @tauri-apps/plugin-dialog

# Install shadcn dependencies
pnpm add -D tailwindcss postcss autoprefixer
pnpm dlx tailwindcss init -p

# Install state management
pnpm add zustand
```

### Step 2: Initialize Shadcn (Tauri v2 Compatible)

```bash
pnpm dlx shadcn@latest init
```

Choose these options:
- Style: Default
- Base color: Neutral
- CSS variables: Yes

### Step 3: Add Required Shadcn Components

```bash
pnpm dlx shadcn@latest add button card input select tabs
```

### Step 4: Create Tauri Client Wrapper

**File**: `src/lib/tauri-client.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { Block, Command, Event } from '../types/models';

export class TauriClient {
  // File operations
  static async openFile(path: string): Promise<string> {
    return await invoke('open_file', { path });
  }

  static async createFile(path: string): Promise<string> {
    return await invoke('create_file', { path });
  }

  static async saveFile(fileId: string): Promise<void> {
    return await invoke('save_file', { fileId });
  }

  static async closeFile(fileId: string): Promise<void> {
    return await invoke('close_file', { fileId });
  }

  static async listOpenFiles(): Promise<string[]> {
    return await invoke('list_open_files');
  }

  // Block operations
  static async executeCommand(fileId: string, cmd: Command): Promise<Event[]> {
    return await invoke('execute_command', { fileId, cmd });
  }

  static async getBlock(fileId: string, blockId: string): Promise<Block> {
    return await invoke('get_block', { fileId, blockId });
  }

  static async getAllBlocks(fileId: string): Promise<Block[]> {
    return await invoke('get_all_blocks', { fileId });
  }
}
```

### Step 5: Create State Store

**File**: `src/store/app-store.ts`

```typescript
import { create } from 'zustand';
import type { Block } from '../types/models';

interface FileState {
  fileId: string | null;
  blocks: Block[];
}

interface AppStore {
  files: Map<string, FileState>;
  currentFileId: string | null;
  selectedBlockId: string | null;
  currentEditorId: string;

  openFile: (fileId: string) => void;
  setBlocks: (fileId: string, blocks: Block[]) => void;
  addBlock: (fileId: string, block: Block) => void;
  updateBlock: (fileId: string, block: Block) => void;
  selectBlock: (id: string | null) => void;
  closeFile: (fileId: string) => void;
  switchFile: (fileId: string) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  files: new Map(),
  currentFileId: null,
  selectedBlockId: null,
  currentEditorId: 'editor-' + Math.random().toString(36).substr(2, 9),

  openFile: (fileId) =>
    set((state) => {
      const newFiles = new Map(state.files);
      newFiles.set(fileId, { fileId, blocks: [] });
      return { files: newFiles, currentFileId: fileId };
    }),

  setBlocks: (fileId, blocks) =>
    set((state) => {
      const newFiles = new Map(state.files);
      const fileState = newFiles.get(fileId);
      if (fileState) {
        fileState.blocks = blocks;
      }
      return { files: newFiles };
    }),

  addBlock: (fileId, block) =>
    set((state) => {
      const newFiles = new Map(state.files);
      const fileState = newFiles.get(fileId);
      if (fileState) {
        fileState.blocks = [...fileState.blocks, block];
      }
      return { files: newFiles };
    }),

  updateBlock: (fileId, block) =>
    set((state) => {
      const newFiles = new Map(state.files);
      const fileState = newFiles.get(fileId);
      if (fileState) {
        fileState.blocks = fileState.blocks.map((b) =>
          b.block_id === block.block_id ? block : b
        );
      }
      return { files: newFiles };
    }),

  selectBlock: (id) => set({ selectedBlockId: id }),

  closeFile: (fileId) =>
    set((state) => {
      const newFiles = new Map(state.files);
      newFiles.delete(fileId);
      const newCurrentFileId =
        state.currentFileId === fileId
          ? newFiles.keys().next().value || null
          : state.currentFileId;
      return { files: newFiles, currentFileId: newCurrentFileId };
    }),

  switchFile: (fileId) => set({ currentFileId: fileId }),
}));
```

### Step 6: Create Block Component

**File**: `src/components/block-card.tsx`

```tsx
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { Block } from '@/types/models';

interface BlockCardProps {
  block: Block;
  onSelect: (id: string) => void;
  isSelected: boolean;
}

export function BlockCard({ block, onSelect, isSelected }: BlockCardProps) {
  return (
    <Card
      className={`cursor-pointer transition-all hover:shadow-md ${
        isSelected ? 'ring-2 ring-primary' : ''
      }`}
      onClick={() => onSelect(block.block_id)}
    >
      <CardHeader>
        <CardTitle className="text-sm">{block.name}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-xs text-muted-foreground mb-1">
          Type: {block.block_type}
        </p>
        <p className="text-xs text-muted-foreground">Owner: {block.owner}</p>
        {Object.keys(block.children).length > 0 && (
          <p className="text-xs text-muted-foreground mt-2">
            Links: {Object.keys(block.children).length}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
```

### Step 7: Create Main App Component

**File**: `src/App.tsx`

```tsx
import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { BlockCard } from '@/components/block-card';
import { TauriClient } from '@/lib/tauri-client';
import { useAppStore } from '@/store/app-store';
import { Command } from '@/types/models';

function App() {
  const {
    files,
    currentFileId,
    selectedBlockId,
    currentEditorId,
    openFile,
    setBlocks,
    selectBlock,
    closeFile,
    switchFile,
  } = useAppStore();

  const currentFile = currentFileId ? files.get(currentFileId) : null;

  const handleOpenFile = async () => {
    try {
      const path = await open({
        filters: [{ name: 'ELF Files', extensions: ['elf'] }],
      });

      if (path) {
        const fileId = await TauriClient.openFile(path as string);
        openFile(fileId);
        const blocks = await TauriClient.getAllBlocks(fileId);
        setBlocks(fileId, blocks);
      }
    } catch (error) {
      console.error('Failed to open file:', error);
      alert('Failed to open file: ' + error);
    }
  };

  const handleCreateBlock = async () => {
    if (!currentFileId) return;

    try {
      const cmd: Command = {
        cmd_id: crypto.randomUUID(),
        editor_id: currentEditorId,
        cap_id: 'core.create',
        block_id: crypto.randomUUID(),
        payload: {
          name: `Block ${(currentFile?.blocks.length || 0) + 1}`,
          block_type: 'markdown',
        },
        timestamp: new Date().toISOString(),
      };

      await TauriClient.executeCommand(currentFileId, cmd);
      const updatedBlocks = await TauriClient.getAllBlocks(currentFileId);
      setBlocks(currentFileId, updatedBlocks);
    } catch (error) {
      console.error('Failed to create block:', error);
      alert('Failed to create block: ' + error);
    }
  };

  const handleCloseFile = async (fileId: string) => {
    try {
      await TauriClient.closeFile(fileId);
      closeFile(fileId);
    } catch (error) {
      console.error('Failed to close file:', error);
    }
  };

  return (
    <div className="container mx-auto p-4">
      <div className="mb-4 flex gap-2">
        <Button onClick={handleOpenFile}>Open File</Button>
        <Button onClick={handleCreateBlock} disabled={!currentFileId}>
          Create Block
        </Button>
      </div>

      {files.size > 0 && (
        <Tabs value={currentFileId || ''} onValueChange={switchFile}>
          <TabsList>
            {Array.from(files.keys()).map((fileId) => (
              <TabsTrigger key={fileId} value={fileId}>
                {fileId.substring(0, 8)}...
                <Button
                  variant="ghost"
                  size="sm"
                  className="ml-2 h-4 w-4 p-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleCloseFile(fileId);
                  }}
                >
                  ×
                </Button>
              </TabsTrigger>
            ))}
          </TabsList>

          {Array.from(files.entries()).map(([fileId, fileState]) => (
            <TabsContent key={fileId} value={fileId}>
              <div className="grid grid-cols-3 gap-4">
                {fileState.blocks.map((block) => (
                  <BlockCard
                    key={block.block_id}
                    block={block}
                    onSelect={selectBlock}
                    isSelected={block.block_id === selectedBlockId}
                  />
                ))}
              </div>
            </TabsContent>
          ))}
        </Tabs>
      )}

      {files.size === 0 && (
        <div className="text-center text-muted-foreground mt-8">
          No files open. Click "Open File" to get started.
        </div>
      )}
    </div>
  );
}

export default App;
```

### Step 8: Update Tauri Config for Dialog Plugin

**File**: `src-tauri/tauri.conf.json`

Ensure the dialog plugin is configured (should be there by default in Tauri v2):

```json
{
  "plugins": {
    "dialog": {
      "all": true
    }
  }
}
```

### Step 9: Test the App

```bash
pnpm tauri dev
```

**Expected**:
1. Can open multiple `.elf` files (tabs appear)
2. Can switch between files via tabs
3. Can create blocks in each file independently
4. Can close files (tab closes, engine shuts down)
5. Blocks are file-scoped (each file has its own blocks)

## Done

Multi-file Tauri v2 app complete:
- ✅ EngineManager integration for multi-file support
- ✅ Tauri v2 plugin-dialog for file picker
- ✅ Tabbed interface for multiple open files
- ✅ File-scoped block operations
- ✅ Clean shutdown per file
- ✅ Shadcn UI components
- ✅ Zustand for multi-file state

**Supports**:
- Multiple `.elf` files open simultaneously
- Independent engine actors per file
- Tab-based UI for file switching
- Proper resource cleanup on file close

**Next steps** (post-MVP):
- Add Tauri events for real-time state updates (`state_changed`)
- Implement markdown editor
- Add drag-and-drop linking between blocks
- Show file paths in tabs (not just IDs)
