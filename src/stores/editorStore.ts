import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Tool = 'select' | 'arrow' | 'rect' | 'text' | 'blur' | 'number';

export interface Shape {
  id: string;
  type: 'arrow' | 'rect' | 'text' | 'blur' | 'number';
  x: number;
  y: number;
  width?: number;
  height?: number;
  points?: number[];
  text?: string;
  number?: number;
  color: string;
  strokeWidth: number;
}

interface EditorState {
  tool: Tool;
  color: string;
  strokeWidth: number;
  shapes: Shape[];
  selectedId: string | null;
  history: Shape[][];
  historyIndex: number;
  nextNumber: number;

  setTool: (tool: Tool) => void;
  setColor: (color: string) => void;
  setStrokeWidth: (width: number) => void;
  addShape: (shape: Shape) => void;
  updateShape: (id: string, updates: Partial<Shape>) => void;
  deleteShape: (id: string) => void;
  setSelectedId: (id: string | null) => void;
  clearShapes: () => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
  getNextNumber: () => number;
}

// Separate store for persisted preferences
interface PreferencesState {
  lastTool: Tool;
  lastColor: string;
  lastStrokeWidth: number;
  setLastTool: (tool: Tool) => void;
  setLastColor: (color: string) => void;
  setLastStrokeWidth: (width: number) => void;
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      lastTool: 'arrow',
      lastColor: '#ff0000',
      lastStrokeWidth: 3,
      setLastTool: (lastTool) => set({ lastTool }),
      setLastColor: (lastColor) => set({ lastColor }),
      setLastStrokeWidth: (lastStrokeWidth) => set({ lastStrokeWidth }),
    }),
    {
      name: 'editor-preferences',
    }
  )
);

export const useEditorStore = create<EditorState>((set, get) => ({
  tool: usePreferencesStore.getState().lastTool,
  color: usePreferencesStore.getState().lastColor,
  strokeWidth: usePreferencesStore.getState().lastStrokeWidth,
  shapes: [],
  selectedId: null,
  history: [[]],
  historyIndex: 0,
  nextNumber: 1,

  setTool: (tool) => {
    set({ tool, selectedId: null });
    usePreferencesStore.getState().setLastTool(tool);
  },
  setColor: (color) => {
    set({ color });
    usePreferencesStore.getState().setLastColor(color);
  },
  setStrokeWidth: (strokeWidth) => {
    set({ strokeWidth });
    usePreferencesStore.getState().setLastStrokeWidth(strokeWidth);
  },

  addShape: (shape) => set((state) => {
    const newShapes = [...state.shapes, shape];
    const newHistory = state.history.slice(0, state.historyIndex + 1);
    newHistory.push(newShapes);
    return {
      shapes: newShapes,
      history: newHistory,
      historyIndex: newHistory.length - 1
    };
  }),

  updateShape: (id, updates) => set((state) => {
    const newShapes = state.shapes.map((s) => s.id === id ? { ...s, ...updates } : s);
    const newHistory = state.history.slice(0, state.historyIndex + 1);
    newHistory.push(newShapes);
    return {
      shapes: newShapes,
      history: newHistory,
      historyIndex: newHistory.length - 1
    };
  }),

  deleteShape: (id) => set((state) => {
    const newShapes = state.shapes.filter((s) => s.id !== id);
    const newHistory = state.history.slice(0, state.historyIndex + 1);
    newHistory.push(newShapes);
    return {
      shapes: newShapes,
      selectedId: state.selectedId === id ? null : state.selectedId,
      history: newHistory,
      historyIndex: newHistory.length - 1
    };
  }),

  setSelectedId: (selectedId) => set({ selectedId }),

  clearShapes: () => set({
    shapes: [],
    selectedId: null,
    history: [[]],
    historyIndex: 0,
    nextNumber: 1
  }),

  undo: () => set((state) => {
    if (state.historyIndex <= 0) return state;
    const newIndex = state.historyIndex - 1;
    return {
      shapes: state.history[newIndex],
      historyIndex: newIndex,
      selectedId: null
    };
  }),

  redo: () => set((state) => {
    if (state.historyIndex >= state.history.length - 1) return state;
    const newIndex = state.historyIndex + 1;
    return {
      shapes: state.history[newIndex],
      historyIndex: newIndex,
      selectedId: null
    };
  }),

  canUndo: () => get().historyIndex > 0,
  canRedo: () => get().historyIndex < get().history.length - 1,

  getNextNumber: () => {
    const num = get().nextNumber;
    set({ nextNumber: num + 1 });
    return num;
  },
}));
