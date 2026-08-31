import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import Range from "@/components/Range.svelte";

/**
 * The slider is a native `input[type=range]` behind custom paint. These tests
 * pin what that choice buys — the slider role, its label and value text, and
 * disabled semantics — plus the input/commit split that keeps a drag from
 * flooding the audio worker with one IPC call per frame.
 *
 * happy-dom does not implement the native control's own key stepping, so the
 * keyboard tests assert the commit contract this component owns rather than the
 * value the browser would have produced.
 */
describe("Range", () => {
  it("exposes the native slider role with its label and value text", () => {
    render(Range, {
      props: { value: 0.5, ariaLabel: "Volume for Spotify", ariaValueText: "50%" },
    });

    const slider = screen.getByRole("slider", { name: "Volume for Spotify" });
    expect(slider).toHaveAttribute("aria-valuetext", "50%");
    expect(slider).toHaveValue("0.5");
  });

  it("announces the muted state, because a level alone would mislead", () => {
    render(Range, {
      props: { value: 0.5, muted: true, ariaLabel: "Volume", ariaValueText: "50%, muted" },
    });
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "50%, muted");
  });

  // `fireEvent` rather than `userEvent` here: user-event routes Home/End and
  // the page keys through `setSelectionRange`, which happy-dom does not
  // implement for a range input and which a real range control ignores anyway.
  it.each(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown"])(
    "commits exactly once when %s is released",
    async (key) => {
      const onCommit = vi.fn();
      render(Range, { props: { value: 0.5, ariaLabel: "Volume", onCommit } });

      const slider = screen.getByRole("slider");
      await fireEvent.keyDown(slider, { key });
      await fireEvent.keyUp(slider, { key });

      expect(onCommit).toHaveBeenCalledTimes(1);
      expect(onCommit).toHaveBeenCalledWith(0.5);
    },
  );

  it("does not commit on a key the slider ignores", async () => {
    const onCommit = vi.fn();
    render(Range, { props: { value: 0.5, ariaLabel: "Volume", onCommit } });

    const slider = screen.getByRole("slider");
    await fireEvent.keyDown(slider, { key: "Escape" });
    await fireEvent.keyUp(slider, { key: "Escape" });

    expect(onCommit).not.toHaveBeenCalled();
  });

  it("reports every drag frame through onInput but commits only on release", async () => {
    const user = userEvent.setup();
    const onInput = vi.fn();
    const onCommit = vi.fn();
    render(Range, { props: { value: 0.5, ariaLabel: "Volume", onInput, onCommit } });

    const slider = screen.getByRole("slider") as HTMLInputElement;

    await user.pointer({ keys: "[MouseLeft>]", target: slider });
    slider.value = "0.8";
    slider.dispatchEvent(new Event("input", { bubbles: true }));

    expect(onInput).toHaveBeenCalledWith(0.8);
    // Nothing has reached the backend yet — that is the whole point of the split.
    expect(onCommit).not.toHaveBeenCalled();

    await user.pointer({ keys: "[/MouseLeft]", target: slider });
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(0.8);
  });

  it("resets to the given value on double-click", async () => {
    const user = userEvent.setup();
    const onCommit = vi.fn();
    render(Range, { props: { value: 0.3, resetTo: 1, ariaLabel: "Volume", onCommit } });

    await user.dblClick(screen.getByRole("slider"));
    expect(onCommit).toHaveBeenCalledWith(1);
  });

  it("disables the control when the session is not controllable", () => {
    render(Range, { props: { value: 0.5, disabled: true, ariaLabel: "Volume" } });
    expect(screen.getByRole("slider")).toBeDisabled();
  });

  it("emits nothing at all while disabled", async () => {
    const user = userEvent.setup();
    const onInput = vi.fn();
    const onCommit = vi.fn();
    render(Range, {
      props: { value: 0.3, disabled: true, resetTo: 1, ariaLabel: "Volume", onInput, onCommit },
    });

    const slider = screen.getByRole("slider") as HTMLInputElement;
    await user.dblClick(slider);
    await user.pointer({ keys: "[MouseLeft>]", target: slider });
    slider.dispatchEvent(new Event("input", { bubbles: true }));
    await user.pointer({ keys: "[/MouseLeft]", target: slider });

    expect(onInput).not.toHaveBeenCalled();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("clamps a value the backend should never have sent", () => {
    render(Range, { props: { value: 5, ariaLabel: "Volume" } });
    // The native input clamps to its own max, so the paint cannot overflow.
    expect(screen.getByRole("slider")).toHaveValue("1");
  });
});
