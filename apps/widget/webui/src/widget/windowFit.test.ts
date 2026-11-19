import { beforeEach, describe, expect, it, vi } from 'vitest';

const ipc = vi.hoisted(() => ({ layoutWidget: vi.fn() }));
vi.mock('../lib/ipc', () => ({ commands: ipc }));

import { createLayout, roomFor, type Box } from './windowFit';

const bubble: Box = { x: 24, y: 24, width: 44, height: 44 };
const pill: Box = { x: 30, y: 26, width: 172, height: 40 };
const hugged: Box = { x: 24, y: 24, width: 172, height: 40 };
const recording = { size: { width: 172, height: 40 }, room: { width: 0, height: 0 }, inset: 8 };
const idle = { size: { width: 44, height: 44 }, room: { width: 0, height: 0 }, inset: 0 };

describe('roomFor', () => {
  it('asks for no room at rest, tooltip room while hovering, menu room while a menu is open', () => {
    expect(roomFor({ hovering: false, menuOpen: false })).toEqual({ width: 0, height: 0 });
    expect(roomFor({ hovering: true, menuOpen: false }).height).toBeGreaterThanOrEqual(120);
    expect(roomFor({ hovering: true, menuOpen: true }).width).toBeGreaterThan(400);
  });
});

describe('createLayout', () => {
  beforeEach(() => {
    ipc.layoutWidget.mockReset();
    ipc.layoutWidget.mockImplementation(async (_size, _room, _inset, current: Box | null) =>
      current ? { from: bubble, to: pill } : { from: hugged, to: hugged },
    );
  });

  it('grows the window around both rects, morphs the pill, then hugs the new pill', async () => {
    const show = vi.fn(async () => {});
    await createLayout(show, () => bubble)(recording);
    expect(ipc.layoutWidget.mock.calls.map((call) => call[3])).toEqual([bubble, null]);
    expect(show.mock.calls).toEqual([
      [bubble, false],
      [pill, true],
      [hugged, false],
    ]);
  });

  it('does nothing again for the target it already laid out', async () => {
    const layout = createLayout(vi.fn(async () => {}), () => bubble);
    await layout(recording);
    await layout({ ...recording, size: { ...recording.size } });
    expect(ipc.layoutWidget).toHaveBeenCalledTimes(2);
  });

  it('cuts a morph short for a newer target, which starts from where the pill is drawn', async () => {
    let drawn: Box = bubble;
    const show = vi.fn((box: Box, animate: boolean) => (animate ? new Promise<void>(() => {}) : Promise.resolve()));
    const layout = createLayout(show, () => drawn);
    void layout(recording);
    await vi.waitFor(() => expect(show).toHaveBeenCalledWith(pill, true));
    drawn = { ...pill, width: 90 };
    void layout(idle);
    await vi.waitFor(() => expect(ipc.layoutWidget).toHaveBeenCalledTimes(2));
    expect(ipc.layoutWidget.mock.calls[1][0]).toEqual(idle.size);
    expect(ipc.layoutWidget.mock.calls[1][3]).toEqual(drawn);
  });
});
