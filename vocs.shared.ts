import path from 'node:path'

import { defineConfig } from 'vocs'

const cwdBase = path.basename(process.cwd())
const rootDir = cwdBase === 'docs' ? '.' : 'docs'
const collapseSidebarItems = (items: any[]): any[] =>
  items.map((item) =>
    item.items
      ? {
          ...item,
          collapsed: item.collapsed ?? true,
          items: collapseSidebarItems(item.items),
        }
      : item,
  )

export default defineConfig({
  aiCta: false,
  rootDir,
  title: 'Edge Language',
  titleTemplate: '%s · Edge Language',
  description: 'A domain specific language for the Ethereum Virtual Machine',
  iconUrl: '/favicon.ico',
  logoUrl: '/logo.png',
  editLink: {
    pattern: 'https://github.com/refcell/edge-rs/edit/main/docs/pages/:path',
    text: 'Edit on GitHub',
  },
  topNav: [
    { text: 'Introduction', link: '/' },
    { text: 'Specifications', link: '/specs/overview' },
    { text: 'Compiler', link: '/compiler/overview' },
    { text: 'GitHub', link: 'https://github.com/refcell/edge-rs' },
  ],
  sidebar: {
    '/': collapseSidebarItems([
      {
        text: 'Introduction',
        link: '/',
      },
      {
        text: 'Specifications',
        items: [
          { text: 'Overview', link: '/specs/overview' },
          {
            text: 'Syntax',
            items: [
              { text: 'Overview', link: '/specs/syntax/overview' },
              { text: 'Comments', link: '/specs/syntax/comments' },
              { text: 'Identifiers', link: '/specs/syntax/identifiers' },
              { text: 'Data Locations', link: '/specs/syntax/locations' },
              { text: 'Expressions', link: '/specs/syntax/expressions' },
              { text: 'Statements', link: '/specs/syntax/statements' },
              { text: 'Variables', link: '/specs/syntax/variables' },
              {
                text: 'Type System',
                items: [
                  { text: 'Overview', link: '/specs/syntax/types/overview' },
                  { text: 'Primitive Types', link: '/specs/syntax/types/primitives' },
                  { text: 'Type Assignment', link: '/specs/syntax/types/assignment' },
                  { text: 'Array Types', link: '/specs/syntax/types/arrays' },
                  { text: 'Product Types', link: '/specs/syntax/types/products' },
                  { text: 'Sum Types', link: '/specs/syntax/types/sum' },
                  { text: 'Generics', link: '/specs/syntax/types/generics' },
                  { text: 'Trait Constraints', link: '/specs/syntax/types/traits' },
                  { text: 'Implementation', link: '/specs/syntax/types/implementation' },
                  { text: 'Function Types', link: '/specs/syntax/types/function' },
                  { text: 'Event Types', link: '/specs/syntax/types/events' },
                  { text: 'ABI', link: '/specs/syntax/types/abi' },
                  { text: 'Contract Objects', link: '/specs/syntax/types/contracts' },
                ],
              },
              {
                text: 'Control Flow',
                items: [
                  { text: 'Overview', link: '/specs/syntax/control/overview' },
                  { text: 'Loops', link: '/specs/syntax/control/loops' },
                  { text: 'Code Block', link: '/specs/syntax/control/code' },
                  { text: 'Branching', link: '/specs/syntax/control/branching' },
                ],
              },
              { text: 'Operators', link: '/specs/syntax/operators' },
              {
                text: 'Compile Time',
                items: [
                  { text: 'Overview', link: '/specs/syntax/compile/overview' },
                  { text: 'Literals', link: '/specs/syntax/compile/literals' },
                  { text: 'Constants', link: '/specs/syntax/compile/constants' },
                  { text: 'Branching', link: '/specs/syntax/compile/branching' },
                  { text: 'Functions', link: '/specs/syntax/compile/functions' },
                ],
              },
              { text: 'Modules', link: '/specs/syntax/modules' },
            ],
          },
          {
            text: 'Syntax Showcase',
            items: [
              { text: 'Overview', link: '/specs/showcase/overview' },
              { text: 'Basics', link: '/specs/showcase/basics' },
              { text: 'ERC20', link: '/specs/showcase/erc20' },
            ],
          },
          {
            text: 'Semantics',
            items: [
              { text: 'Overview', link: '/specs/semantics/overview' },
              { text: 'Codesize', link: '/specs/semantics/codesize' },
              { text: 'Namespaces', link: '/specs/semantics/namespaces' },
              { text: 'Scoping', link: '/specs/semantics/scoping' },
              { text: 'Visibility', link: '/specs/semantics/visibility' },
            ],
          },
          { text: 'Inline Assembly', link: '/specs/inline' },
          { text: 'Built-In', link: '/specs/builtins' },
        ],
      },
      {
        text: 'Compiler',
        items: [
          { text: 'Architecture', link: '/compiler/overview' },
          { text: 'Quickstart', link: '/compiler/quickstart' },
        ],
      },
      {
        text: 'Tooling',
        link: '/tools/overview',
      },
      {
        text: 'Contributing',
        link: '/contributing/contributing',
      },
      {
        text: 'Contact',
        link: '/contact/contact',
      },
    ]),
  },
})
