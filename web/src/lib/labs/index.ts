// Lab views: whole-screen experiments that do not fit the Library layout,
// such as drawing a store's graph. Each one is a Svelte component here,
// added to the list below; the Lab module lists them and gives the chosen
// one the whole screen.
//
// A lab view gets the open store and the image under the cursor in the
// Library (both may be null), so an experiment can start from wherever the
// person was. It reads what it needs itself, through ../api.ts or the
// server's routes, and changes nothing in Pan unless that is the experiment.

import type { Component } from 'svelte'
import type { Store } from '../api'
import Joystick from './Joystick.svelte'
import Atlas from './Atlas.svelte'

export interface LabProps {
  store: Store | null
  image: string | null
  /** Show this image in the Library, full size, in "everything". */
  onopen: (id: string) => void
}

export interface LabView {
  id: string
  title: string
  /** One line: what the view shows and what it is for. */
  about: string
  view: Component<LabProps>
}

export const labs: LabView[] = [
  {
    id: 'atlas',
    title: 'Atlas',
    about: 'Every image as a dot, placed by look and coloured by when; zoom in and the dots become pictures',
    view: Atlas,
  },
  {
    id: 'joystick',
    title: 'Joystick',
    about: 'Steer through the store by look: eight directions from any image, along the ways its neighbours differ most',
    view: Joystick,
  },
]
