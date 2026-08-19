import { Routes, Route } from 'react-router'
import Layout from './pages/Layout'
import Home from './pages/Home'
import Docs from './pages/Docs'
import Pricing from './pages/Pricing'
import Enterprise from './pages/Enterprise'
import Proof from './pages/Proof'
import Trust from './pages/Trust'

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<Home />} />
        <Route path="/docs" element={<Docs />} />
        <Route path="/pricing" element={<Pricing />} />
        <Route path="/enterprise" element={<Enterprise />} />
        <Route path="/proof" element={<Proof />} />
        <Route path="/trust" element={<Trust />} />
      </Route>
    </Routes>
  )
}
