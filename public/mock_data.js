// Mock data for Pan instances, images, photosets, folders, and RDF triples

export const PAN_INSTANCES = [
  {
    id: 'studio-2026',
    name: 'Studio 2026 (Active Shoots)',
    mode: 'Mode 2 (Referenced Index)',
    port: 7401,
    status: 'online',
    itemCount: '3,420 photos',
    storagePath: '/Volumes/Shoot_NVMe_01/2026_Studio',
    color: '#06b6d4',
    description: 'Current high-speed commercial dance & portrait sessions.'
  },
  {
    id: 'primary-archive',
    name: 'Primary Historical Archive',
    mode: 'Mode 2 (Referenced Index)',
    port: 7402,
    status: 'online',
    itemCount: '1.2M photos',
    storagePath: '/Volumes/Archive_RAID_01/1998_2025',
    color: '#8b5cf6',
    description: 'Complete 25-year cold storage catalog with 4K cached previews.'
  },
  {
    id: 'horae-pool',
    name: 'Horae Generative Pool',
    mode: 'Mode 1 (Managed Store)',
    port: 7403,
    status: 'online',
    itemCount: '8,450 renders',
    storagePath: '/Users/rob/pan/managed/horae',
    color: '#ec4899',
    description: 'Agent-generated synthetic renders, LoRA outputs, and PNG chunks.'
  }
];

export const MOCK_SHOOTS = [
  { id: '2026/05_studio_dance', name: '2026/05_studio_dance', count: 42, year: '2026' },
  { id: '2026/06_rehearsal_hall', name: '2026/06_rehearsal_hall', count: 28, year: '2026' },
  { id: '2026/07_rooftop_editorial', name: '2026/07_rooftop_editorial', count: 35, year: '2026' },
  { id: '2025/11_stage_lights', name: '2025/11_stage_lights', count: 64, year: '2025' },
  { id: '2025/08_outdoor_golden', name: '2025/08_outdoor_golden', count: 51, year: '2025' }
];

export const MOCK_PHOTOSETS = [
  {
    id: 's_dance_portfolio_2026',
    name: '2026 Dance Studio Portfolio',
    description: 'Curated hero selections from spring studio sessions on pointe.',
    diskXml: 'photosets/s_dance_portfolio_2026.xml',
    coverImage: 'img_01',
    itemCount: 6,
    tags: ['dance', 'studio', 'portfolio', 'hero']
  },
  {
    id: 's_motion_leaps',
    name: 'Hero Leaps & Silhouettes',
    description: 'High-speed motor-drive sequences capturing mid-air hang time.',
    diskXml: 'photosets/s_motion_leaps.xml',
    coverImage: 'img_04',
    itemCount: 4,
    tags: ['action', 'split', 'high-speed', 'freeze']
  },
  {
    id: 's_bw_editorial',
    name: 'Monochrome High-Contrast',
    description: 'Deep low-key studio lighting with chiaroscuro rim accents.',
    diskXml: 'photosets/s_bw_editorial.xml',
    coverImage: 'img_06',
    itemCount: 5,
    tags: ['black-and-white', 'dramatic', 'contrast', 'rim-light']
  }
];

export const MOCK_SMART_COLLECTIONS = [
  { id: 'c_five_star_picks', name: '5★ Picked Heroes', sparql: 'SELECT ?s WHERE { ?s pan:rating 5 ; pan:isPicked true . }', icon: '★' },
  { id: 'c_golden_hour', name: 'Golden Hour / Natural Light', sparql: 'SELECT ?s WHERE { ?s pan:sceneLighting "golden hour natural light" . }', icon: '☀' },
  { id: 'c_hero_outtakes', name: 'Unpicked Outtakes of Heroes', sparql: 'SELECT ?s WHERE { ?s pan:isPicked false ; pan:burstCount ?c . FILTER(?c > 5) }', icon: '⧉' },
  { id: 'c_high_appeal', name: 'Aesthetic Score > 90', sparql: 'SELECT ?s WHERE { ?s pan:aestheticAppeal ?a . FILTER(?a >= 90) }', icon: '◆' }
];

// Helper to create visual SVG placeholder thumbnails with dynamic colors
function makeSvg(text, bg1, bg2, accent) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300" width="100%" height="100%">
    <defs>
      <linearGradient id="g_${text.replace(/[^a-zA-Z0-9]/g, '')}" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="${bg1}" />
        <stop offset="100%" stop-color="${bg2}" />
      </linearGradient>
    </defs>
    <rect width="400" height="300" fill="url(#g_${text.replace(/[^a-zA-Z0-9]/g, '')})" />
    <circle cx="200" cy="120" r="45" fill="${accent}" opacity="0.35" />
    <path d="M120 250 L180 160 L240 220 L320 130 L380 250 Z" fill="${accent}" opacity="0.25" />
    <text x="20" y="40" fill="#ffffff" font-family="-apple-system, sans-serif" font-size="14" font-weight="700" opacity="0.9">${text}</text>
    <text x="20" y="275" fill="#ffffff" font-family="-apple-system, sans-serif" font-size="11" opacity="0.6">4K RAW PREVIEW · DISPLAY P3</text>
  </svg>`;
  return 'data:image/svg+xml;utf8,' + encodeURIComponent(svg);
}

export const MOCK_IMAGES = [
  {
    id: 'img_01',
    filename: 'IMG_20260514_0142.CR3',
    folderPath: '2026/05_studio_dance',
    shootName: '2026_05_studio_dance',
    year: '2026',
    dateCreated: '2026-05-14T14:22:10-07:00',
    rating: 5,
    isPicked: true,
    isRejected: false,
    photosets: ['s_dance_portfolio_2026'],
    burstId: 'burst_20260514_014',
    burstCount: 12,
    burstIndex: 3,
    subject: 'Sarah Chen',
    previewUrl: makeSvg('IMG_0142 · Grand Jeté Peak', '#111827', '#1f2937', '#06b6d4'),
    exif: {
      camera: 'Canon EOS R5',
      lens: 'RF 85mm f/1.2L USM',
      focalLength: '85mm',
      aperture: 'f/1.4',
      shutter: '1/1600s',
      iso: 400
    },
    proxy: {
      shortDescription: 'Solo female ballerina performing a grand jeté at apex height in a dark studio setting.',
      longDescription: 'Dramatic low-key studio lighting casts an intense rim light along the dancer’s extended limbs and arched spine. Fine powdered chalk particles suspend around her pointe shoes in freeze motion.',
      sceneObjects: ['ballerina', 'tutu', 'pointe shoes', 'chalk dust', 'dark studio floor'],
      sceneLighting: 'dramatic rim light with soft fill',
      sceneMood: 'graceful intensity',
      aestheticConsensus: {
        technical: 95.0,
        appeal: 96.0,
        mean: 95.5,
        critique: 'Critical focus nailed on front eye pupil; exceptional shutter freeze on airborne chalk dust with zero motion blur.'
      },
      poseMetrics: {
        keypointCount: 133,
        confidence: 0.99,
        postureType: 'grand jeté attitude',
        anatomicalIntegrity: '100% verified (5 fingers each hand, clean joint bounds)'
      },
      depthMetrics: {
        profile: 'strong foreground subject isolation',
        groundContact: false,
        separationRatio: '0.88'
      },
      colorMetrics: {
        dominantPalette: ['#121824', '#F4EBE2', '#4B7B9E'],
        gamut: 'display-p3'
      }
    }
  },
  {
    id: 'img_02',
    filename: 'IMG_20260514_0143.CR3',
    folderPath: '2026/05_studio_dance',
    shootName: '2026_05_studio_dance',
    year: '2026',
    dateCreated: '2026-05-14T14:22:11-07:00',
    rating: 4,
    isPicked: false,
    isRejected: false,
    photosets: ['s_dance_portfolio_2026'],
    burstId: 'burst_20260514_014',
    burstCount: 12,
    burstIndex: 4,
    subject: 'Sarah Chen',
    previewUrl: makeSvg('IMG_0143 · Grand Jeté Landing', '#1e1b4b', '#311042', '#a855f7'),
    exif: {
      camera: 'Canon EOS R5',
      lens: 'RF 85mm f/1.2L USM',
      focalLength: '85mm',
      aperture: 'f/1.4',
      shutter: '1/1600s',
      iso: 400
    },
    proxy: {
      shortDescription: 'Ballerina descending from jump onto left toe, right leg extended in arabesque.',
      longDescription: 'Slightly later in the motor-drive sequence; movement decelerating with subtle fabric flutter in the silk skirt.',
      sceneObjects: ['ballerina', 'pointe shoes', 'silk dress'],
      sceneLighting: 'directional studio strobe',
      sceneMood: 'poised landing',
      aestheticConsensus: {
        technical: 89.0,
        appeal: 86.0,
        mean: 87.5,
        critique: 'Sharp focus, but head tilt creates minor shadow under chin.'
      },
      poseMetrics: {
        keypointCount: 133,
        confidence: 0.96,
        postureType: 'arabesque descent',
        anatomicalIntegrity: '100% verified'
      },
      depthMetrics: {
        profile: 'shallow depth of field',
        groundContact: true,
        separationRatio: '0.82'
      },
      colorMetrics: {
        dominantPalette: ['#1A1728', '#E6D3C4', '#78427F'],
        gamut: 'display-p3'
      }
    }
  },
  {
    id: 'img_03',
    filename: 'IMG_20260514_0188.CR3',
    folderPath: '2026/05_studio_dance',
    shootName: '2026_05_studio_dance',
    year: '2026',
    dateCreated: '2026-05-14T14:48:32-07:00',
    rating: 5,
    isPicked: true,
    isRejected: false,
    photosets: ['s_dance_portfolio_2026', 's_bw_editorial'],
    burstId: null,
    burstCount: 1,
    burstIndex: 1,
    subject: 'Sarah Chen',
    previewUrl: makeSvg('IMG_0188 · Seated Mirror Portrait', '#18181b', '#27272a', '#e4e4e7'),
    exif: {
      camera: 'Canon EOS R5',
      lens: 'RF 50mm f/1.2L USM',
      focalLength: '50mm',
      aperture: 'f/1.8',
      shutter: '1/320s',
      iso: 200
    },
    proxy: {
      shortDescription: 'Intimate monochrome portrait of dancer resting on studio floor reflecting in large mirror.',
      longDescription: 'High-contrast black and white grading emphasizing sweat sheen, ribbed leotard texture, and contemplative gaze towards the floor.',
      sceneObjects: ['dancer', 'studio mirror', 'leotard', 'reflection'],
      sceneLighting: 'low-key single overhead softbox',
      sceneMood: 'contemplative, quiet',
      aestheticConsensus: {
        technical: 96.0,
        appeal: 98.0,
        mean: 97.0,
        critique: 'Masterful tonal separation across midtones; emotional resonance; pinpoint optical acuity.'
      },
      poseMetrics: {
        keypointCount: 133,
        confidence: 0.98,
        postureType: 'seated cross-legged hand-on-knee',
        anatomicalIntegrity: '100% verified'
      },
      depthMetrics: {
        profile: 'layered dual reflection depth',
        groundContact: true,
        separationRatio: '0.91'
      },
      colorMetrics: {
        dominantPalette: ['#0A0A0A', '#7A7A7A', '#F0F0F0'],
        gamut: 'display-p3'
      }
    }
  },
  {
    id: 'img_04',
    filename: 'DSC_20260602_0411.ARW',
    folderPath: '2026/06_rehearsal_hall',
    shootName: '2026_06_rehearsal_hall',
    year: '2026',
    dateCreated: '2026-06-02T11:15:04-07:00',
    rating: 4,
    isPicked: true,
    isRejected: false,
    photosets: ['s_motion_leaps'],
    burstId: 'burst_20260602_041',
    burstCount: 18,
    burstIndex: 9,
    subject: 'Elena Rostova',
    previewUrl: makeSvg('DSC_0411 · Rehearsal Split Jump', '#042f2e', '#134e4a', '#14b8a6'),
    exif: {
      camera: 'Sony Alpha 1',
      lens: 'FE 70-200mm f/2.8 GM OSS II',
      focalLength: '135mm',
      aperture: 'f/2.8',
      shutter: '1/2000s',
      iso: 800
    },
    proxy: {
      shortDescription: 'Dancer mid-flight across sun-drenched rehearsal studio with tall industrial windows.',
      longDescription: 'Natural morning sunlight cuts through dusty rehearsal hall air. The dancer executes a dynamic horizontal split suspended between two beams of light.',
      sceneObjects: ['dancer', 'rehearsal barre', 'industrial windows', 'wooden floor'],
      sceneLighting: 'harsh volumetric window sunbeams',
      sceneMood: 'energetic, raw',
      aestheticConsensus: {
        technical: 93.0,
        appeal: 91.0,
        mean: 92.0,
        critique: 'Stunning capture of volumetric sunbeams. Minor sensor dust spot in top right corner easily removed.'
      },
      poseMetrics: {
        keypointCount: 133,
        confidence: 0.97,
        postureType: 'horizontal straddle jump',
        anatomicalIntegrity: '100% verified'
      },
      depthMetrics: {
        profile: 'deep environmental perspective',
        groundContact: false,
        separationRatio: '0.85'
      },
      colorMetrics: {
        dominantPalette: ['#1C2A28', '#D8C3A5', '#E98074'],
        gamut: 'display-p3'
      }
    }
  },
  {
    id: 'img_05',
    filename: 'DSC_20260602_0412.ARW',
    folderPath: '2026/06_rehearsal_hall',
    shootName: '2026_06_rehearsal_hall',
    year: '2026',
    dateCreated: '2026-06-02T11:15:05-07:00',
    rating: 3,
    isPicked: false,
    isRejected: false,
    photosets: ['s_motion_leaps'],
    burstId: 'burst_20260602_041',
    burstCount: 18,
    burstIndex: 10,
    subject: 'Elena Rostova',
    previewUrl: makeSvg('DSC_0412 · Rehearsal Split Outtake', '#064e3b', '#065f46', '#34d399'),
    exif: {
      camera: 'Sony Alpha 1',
      lens: 'FE 70-200mm f/2.8 GM OSS II',
      focalLength: '135mm',
      aperture: 'f/2.8',
      shutter: '1/2000s',
      iso: 800
    },
    proxy: {
      shortDescription: 'Dancer touching down following jump; arms still aloft in rehearsal space.',
      longDescription: 'Sister burst outtake of DSC_0411. Demonstrates slight balance wobble on landing foot.',
      sceneObjects: ['dancer', 'rehearsal barre'],
      sceneLighting: 'window sunbeams',
      sceneMood: 'rehearsal motion',
      aestheticConsensus: {
        technical: 85.0,
        appeal: 78.0,
        mean: 81.5,
        critique: 'Frame captured slightly after peak moment; landing stance lacks apex poise.'
      },
      poseMetrics: {
        keypointCount: 133,
        confidence: 0.95,
        postureType: 'landing recovery',
        anatomicalIntegrity: '100% verified'
      },
      depthMetrics: {
        profile: 'deep perspective',
        groundContact: true,
        separationRatio: '0.80'
      },
      colorMetrics: {
        dominantPalette: ['#1C2A28', '#D8C3A5', '#729888'],
        gamut: 'display-p3'
      }
    }
  },
  {
    id: 'img_06',
    filename: 'DSC_20260719_0891.ARW',
    folderPath: '2026/07_rooftop_editorial',
    shootName: '2026_07_rooftop_editorial',
    year: '2026',
    dateCreated: '2026-07-19T19:40:12-07:00',
    rating: 5,
    isPicked: true,
    isRejected: false,
    photosets: ['s_dance_portfolio_2026', 's_bw_editorial'],
    burstId: null,
    burstCount: 1,
    burstIndex: 1,
    subject: 'Sarah Chen',
    previewUrl: makeSvg('DSC_0891 · Rooftop Twilight Silhouette', '#1c1917', '#292524', '#f59e0b'),
    exif: {
      camera: 'Sony Alpha 1',
      lens: 'FE 24-70mm f/2.8 GM II',
      focalLength: '35mm',
      aperture: 'f/2.8',
      shutter: '1/500s',
      iso: 160
    },
    proxy: {
      shortDescription: 'Ballerina poised in arabesque on concrete rooftop parapet against city skyline at dusk.',
      longDescription: 'Golden sunset fading into deep indigo twilight. The architectural geometry of skyscrapers frames the organic silhouette of the dancer.',
      sceneObjects: ['ballerina', 'rooftop parapet', 'skyscrapers', 'dusk sky'],
      sceneLighting: 'golden hour rim light fading to twilight',
      sceneMood: 'dramatic urban solitude',
      aestheticConsensus: {
        technical: 97.0,
        appeal: 99.0,
        mean: 98.0,
        critique: 'Breathtaking contrast of delicate human form against monumental brutalist architecture; optimal edge sharpening.'
      },
      poseMetrics: {
        keypointCount: 133,
        confidence: 0.99,
        postureType: 'high arabesque en pointe',
        anatomicalIntegrity: '100% verified'
      },
      depthMetrics: {
        profile: 'extreme multi-tier depth (rooftop edge to distant skyline)',
        groundContact: true,
        separationRatio: '0.96'
      },
      colorMetrics: {
        dominantPalette: ['#121016', '#E28743', '#1F355B'],
        gamut: 'display-p3'
      }
    }
  }
];

// RDF Triples for Graph View
export const MOCK_GRAPH_TRIPLES = [
  // Images to Shoots
  { subject: 'img_01', predicate: 'pan:inShoot', object: 'shoot:2026_05_studio_dance' },
  { subject: 'img_02', predicate: 'pan:inShoot', object: 'shoot:2026_05_studio_dance' },
  { subject: 'img_03', predicate: 'pan:inShoot', object: 'shoot:2026_05_studio_dance' },
  { subject: 'img_04', predicate: 'pan:inShoot', object: 'shoot:2026_06_rehearsal_hall' },
  { subject: 'img_05', predicate: 'pan:inShoot', object: 'shoot:2026_06_rehearsal_hall' },
  { subject: 'img_06', predicate: 'pan:inShoot', object: 'shoot:2026_07_rooftop_editorial' },

  // Images to Photosets
  { subject: 'img_01', predicate: 'pan:inPhotoset', object: 'set:s_dance_portfolio_2026' },
  { subject: 'img_02', predicate: 'pan:inPhotoset', object: 'set:s_dance_portfolio_2026' },
  { subject: 'img_03', predicate: 'pan:inPhotoset', object: 'set:s_dance_portfolio_2026' },
  { subject: 'img_06', predicate: 'pan:inPhotoset', object: 'set:s_dance_portfolio_2026' },
  { subject: 'img_04', predicate: 'pan:inPhotoset', object: 'set:s_motion_leaps' },
  { subject: 'img_05', predicate: 'pan:inPhotoset', object: 'set:s_motion_leaps' },
  { subject: 'img_03', predicate: 'pan:inPhotoset', object: 'set:s_bw_editorial' },
  { subject: 'img_06', predicate: 'pan:inPhotoset', object: 'set:s_bw_editorial' },

  // Images to Subjects
  { subject: 'img_01', predicate: 'pan:hasSubject', object: 'subject:sarah_chen' },
  { subject: 'img_02', predicate: 'pan:hasSubject', object: 'subject:sarah_chen' },
  { subject: 'img_03', predicate: 'pan:hasSubject', object: 'subject:sarah_chen' },
  { subject: 'img_06', predicate: 'pan:hasSubject', object: 'subject:sarah_chen' },
  { subject: 'img_04', predicate: 'pan:hasSubject', object: 'subject:elena_rostova' },
  { subject: 'img_05', predicate: 'pan:hasSubject', object: 'subject:elena_rostova' },

  // Images to Poses
  { subject: 'img_01', predicate: 'pan:poseCategory', object: 'pose:grand_jete' },
  { subject: 'img_02', predicate: 'pan:poseCategory', object: 'pose:arabesque' },
  { subject: 'img_03', predicate: 'pan:poseCategory', object: 'pose:seated_rest' },
  { subject: 'img_04', predicate: 'pan:poseCategory', object: 'pose:straddle_jump' },
  { subject: 'img_05', predicate: 'pan:poseCategory', object: 'pose:landing' },
  { subject: 'img_06', predicate: 'pan:poseCategory', object: 'pose:arabesque' },

  // Images to Cameras / Lenses
  { subject: 'img_01', predicate: 'pan:hasLens', object: 'lens:rf_85mm_f12' },
  { subject: 'img_02', predicate: 'pan:hasLens', object: 'lens:rf_85mm_f12' },
  { subject: 'img_03', predicate: 'pan:hasLens', object: 'lens:rf_50mm_f12' },
  { subject: 'img_04', predicate: 'pan:hasLens', object: 'lens:fe_70_200mm_f28' },
  { subject: 'img_05', predicate: 'pan:hasLens', object: 'lens:fe_70_200mm_f28' },
  { subject: 'img_06', predicate: 'pan:hasLens', object: 'lens:fe_24_70mm_f28' }
];
