// What the report designer (ReportDesigner.svelte) shares with the ribbon of
// the Reports tab, as the window designer shares pageDesignerStore: whether
// it can undo, redo and paste, how many parts are picked, whether it
// snaps, shows its preview or its XML; and the designer's own actions,
// which the ribbon's buttons run. Nothing here while no report is open.

export type ReportAlign = "left" | "center" | "right" | "top" | "middle" | "bottom" | "width" | "height" | "across" | "down";

export type ReportActions = {
  undo: () => void;
  redo: () => void;
  cut: () => void;
  copy: () => void;
  paste: () => void;
  remove: () => void;
  align: (how: ReportAlign) => void;
  /** Opens the report's own properties: its paper, margin, stylesheet and data. */
  showReport: () => void;
  /** Deletes the report, after asking. */
  deleteReport: () => void;
};

class ReportDesignerStore {
  /** The open designer's actions; none while no report is open. */
  actions = $state<ReportActions | null>(null);
  canUndo = $state(false);
  canRedo = $state(false);
  canPaste = $state(false);
  /** Parts picked, or 1 for a picked band. */
  picked = $state(0);
  /** The grid parts snap to, in millimetres; 0 for none. */
  grid = $state(1);
  /** The grid's dots show on the paper, apart from whether parts snap to it. */
  showGrid = $state(true);
  previewing = $state(false);
  /** The slide-out shows the report's XML. */
  xml = $state(false);
}

export const reportDesigner = new ReportDesignerStore();
