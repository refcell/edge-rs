#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const docsRoot = path.join(repoRoot, 'docs', 'pages')
const distRoot = path.join(repoRoot, 'docs', 'dist')

const issues = []

function walk(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  const files = []
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...walk(fullPath))
      continue
    }
    if (entry.isFile() && /\.(md|mdx)$/.test(entry.name)) {
      files.push(fullPath)
    }
  }
  return files
}

function getRoute(filePath) {
  const rel = path.relative(docsRoot, filePath).replace(/\\/g, '/')
  if (/^index\.mdx?$/.test(rel)) return '/'
  if (/\/index\.mdx?$/.test(rel)) {
    const dir = rel.replace(/\/index\.mdx?$/, '')
    return `/${dir}`
  }
  return `/${rel.replace(/\.(md|mdx)$/, '')}`
}

function splitFrontmatter(text) {
  if (!text.startsWith('---\n')) return { frontmatter: null, body: text }
  const end = text.indexOf('\n---\n', 4)
  if (end === -1) return { frontmatter: null, body: text }
  return {
    frontmatter: text.slice(4, end),
    body: text.slice(end + 5),
  }
}

function stripCodeFences(text) {
  const lines = text.split('\n')
  const kept = []
  let inFence = false
  for (const line of lines) {
    if (line.startsWith('```')) {
      inFence = !inFence
      continue
    }
    if (!inFence) kept.push(line)
  }
  return kept.join('\n')
}

function normalizeRoute(route) {
  if (!route) return '/'
  const cleaned = route.replace(/\/+$/, '')
  return cleaned === '' ? '/' : cleaned
}

const files = walk(docsRoot)
const routes = new Map()
for (const filePath of files) {
  routes.set(getRoute(filePath), filePath)
}

for (const filePath of files) {
  const rel = path.relative(repoRoot, filePath).replace(/\\/g, '/')
  const route = getRoute(filePath)
  const raw = fs.readFileSync(filePath, 'utf8')
  const { frontmatter, body } = splitFrontmatter(raw)
  const isIndex = route === '/'

  if (!isIndex && frontmatter == null) {
    issues.push(`${rel}: missing frontmatter`)
  }

  const text = stripCodeFences(body)
  const lines = text.split('\n')

  const headingLevels = []
  for (const line of lines) {
    const match = /^(#{1,6})\s+/.exec(line)
    if (match) headingLevels.push(match[1].length)
  }

  const h1Count = headingLevels.filter((level) => level === 1).length
  if (!isIndex && h1Count !== 1) {
    issues.push(`${rel}: expected exactly one H1, found ${h1Count}`)
  }

  let previousHeading = 0
  for (const level of headingLevels) {
    if (previousHeading > 0 && level > previousHeading + 1) {
      issues.push(`${rel}: heading jump from H${previousHeading} to H${level}`)
      break
    }
    previousHeading = level
  }

  if (/\[[^\]]+\]\([^)]+\.md(?:#[^)]+)?\)/.test(text)) {
    issues.push(`${rel}: contains .md links`)
  }

  if (/^TODO\.\.\.$/m.test(text) || /^>\s*todo\b/im.test(text)) {
    issues.push(`${rel}: contains raw TODO placeholder text`)
  }

  const internalLinks = [...text.matchAll(/\[[^\]]+\]\((\/[^)#?]+)(?:#[^)]+)?\)/g)]
  for (const [, href] of internalLinks) {
    const normalized = normalizeRoute(href)
    if (!routes.has(normalized)) {
      issues.push(`${rel}: broken internal link ${href}`)
    }
  }
}

if (fs.existsSync(distRoot)) {
  for (const route of routes.keys()) {
    const outputPath =
      route === '/'
        ? path.join(distRoot, 'index.html')
        : path.join(distRoot, route.slice(1), 'index.html')
    const fallbackPath = path.join(distRoot, `${route.slice(1)}.html`)
    const htmlPath = fs.existsSync(outputPath) ? outputPath : fallbackPath

    if (!fs.existsSync(htmlPath)) {
      issues.push(`dist: missing rendered page for route ${route}`)
      continue
    }

    const html = fs.readFileSync(htmlPath, 'utf8')
    if (!/<h1[\s>]/i.test(html) && route !== '/') {
      issues.push(`dist: rendered page for ${route} is missing an <h1>`)
    }
    if (!/<title>.*<\/title>/i.test(html)) {
      issues.push(`dist: rendered page for ${route} is missing a <title>`)
    }
  }
}

if (issues.length > 0) {
  console.error('Vocs docs validation failed:\n')
  for (const issue of issues) {
    console.error(`- ${issue}`)
  }
  process.exit(1)
}

console.log(`Validated ${files.length} docs source files${fs.existsSync(distRoot) ? ' and dist output' : ''}.`)
