import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'

function App() {
  return (
    <main className="flex min-h-svh items-center justify-center p-6">
      <Card className="w-full max-w-md text-center">
        <CardHeader>
          <CardDescription>siniscalco</CardDescription>
          <CardTitle className="text-4xl">Hello World!</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground">
            Minimal homepage built with shadcn components.
          </p>
        </CardContent>
        <CardFooter className="justify-center">
          <Button size="lg">Hello World!</Button>
        </CardFooter>
      </Card>
    </main>
  )
}

export default App
