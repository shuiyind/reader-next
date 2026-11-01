import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const source = readFileSync(resolve(process.cwd(), 'src/views/ReaderView.vue'), 'utf8')
describe('ReaderView summary management UI', () => {
 it('owns the summary management tabs and prompt editor', () => {
  expect(source).toContain('summary')
  expect(source).toContain('relationships')
  expect(source).toContain('map')
  expect(source).toContain("'settings'")
 expect(source).toContain('chapterSummaryConfigDraft')
 expect(source).toContain('saveChapterSummaryGenerationSettings')
  expect(source).toContain('Number.isFinite')
 expect(source).toContain('保存后端模型')
 })

 it('does not keep the old inline collapse interaction', () => {
  expect(source).not.toContain('chapterSummaryExpanded')
  expect(source).not.toContain("chapterSummaryExpanded ? '收起'")
  expect(source).not.toContain('@click="chapterSummaryExpanded = !chapterSummaryExpanded"')
 })

 it('keeps an entry point when side placement collapses', () => {
  expect(source).toContain('showCollapsedAiPanel')
  expect(source).toContain('chapter-summary-collapsed-pill')
  expect(source).toContain('@click="expandCollapsedAiPanel"')
 })

 it('hides the AI panel through a helper that clears pending generation', () => {
  expect(source).toContain('function hideAiPanel()')
  expect(source).toContain('clearChapterSummaryTimer()')
  expect(source).not.toContain('@click="showChapterSummary = false"')
  expect(source).toContain('function toggleAiPanel()')
 })
})
