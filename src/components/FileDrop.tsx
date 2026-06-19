import { forwardRef } from "react";

interface Props {
  onFile: (file: File) => void;
  children?: React.ReactNode;
}

/** Wraps content in a drag-and-drop target plus a hidden file input (exposed via ref). */
export const FileDrop = forwardRef<HTMLInputElement, Props>(
  function FileDrop({ onFile, children }, ref) {
    return (
      <div
        className="drop-zone"
        onDragOver={(e) => {
          e.preventDefault();
        }}
        onDrop={(e) => {
          e.preventDefault();
          const f = e.dataTransfer.files?.[0];
          if (f && f.type === "image/png") onFile(f);
        }}
      >
        <input
          ref={ref}
          type="file"
          accept="image/png"
          style={{ display: "none" }}
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) onFile(f);
            e.target.value = "";
          }}
        />
        {children}
      </div>
    );
  },
);
